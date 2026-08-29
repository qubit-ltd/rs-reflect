//! Parsing of derive, trait, and impl declarations.

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::spanned::Spanned;
use syn::{
    Data, DeriveInput, Fields, FnArg, GenericParam, Generics, ImplItem, Item, ItemImpl, ItemTrait,
    Pat, ReturnType, TraitItem, Visibility,
};

use crate::ir::{
    AssociatedConstIr, AssociatedTypeIr, DeclarationIr, FieldIr, GenericKindIr, GenericParamIr,
    GenericsIr, HelperAttributeIr, HelperTarget, ImplDeclarationIr, MacroKind, MethodIr,
    ParsedDeclaration, ReturnTypeIr, TraitDeclarationIr, TypeDeclarationIr, TypeDeclarationKindIr,
    VariantIr, VariantKindIr, VisibilityIr,
};
use crate::parse::attributes::{
    ErrorCollector, parse_attributes, parse_helper_tokens, remove_reflect_attributes,
};
use crate::parse::type_ir::{convert_path, convert_type};

/// Parses a procedural macro invocation into shared declaration IR.
///
/// Returns combined syntax diagnostics when the input target or helper grammar is invalid.
pub(crate) fn parse_declaration(
    kind: MacroKind,
    args: TokenStream,
    input: TokenStream,
) -> syn::Result<ParsedDeclaration> {
    match kind {
        MacroKind::Derive => parse_derive(args, input),
        MacroKind::Trait => parse_trait(args, input),
        MacroKind::Impl => parse_impl(args, input),
    }
}

/// Parses a `Reflect` derive input.
fn parse_derive(args: TokenStream, input: TokenStream) -> syn::Result<ParsedDeclaration> {
    if !args.is_empty() {
        return Err(syn::Error::new_spanned(
            args,
            "`Reflect` derive does not accept macro arguments",
        ));
    }
    let input: DeriveInput = syn::parse2(input)?;
    let mut errors = ErrorCollector::default();
    let attributes = parse_attributes(&input.attrs, HelperTarget::Type, &mut errors);
    let generics = convert_generics(&input.generics);
    let visibility = convert_visibility(&input.vis);
    let retained_tokens = input.to_token_stream();
    let declaration = match &input.data {
        Data::Struct(data) => TypeDeclarationIr {
            name: input.ident.clone(),
            kind: TypeDeclarationKindIr::Struct,
            visibility,
            generics,
            attributes,
            fields: convert_fields(&data.fields, &mut errors),
            variants: Vec::new(),
            retained_tokens,
            span: input.ident.span(),
        },
        Data::Enum(data) => TypeDeclarationIr {
            name: input.ident.clone(),
            kind: TypeDeclarationKindIr::Enum,
            visibility,
            generics,
            attributes,
            fields: Vec::new(),
            variants: data
                .variants
                .iter()
                .enumerate()
                .map(|(index, variant)| {
                    let attributes =
                        parse_attributes(&variant.attrs, HelperTarget::Variant, &mut errors);
                    let kind = match variant.fields {
                        Fields::Unit => VariantKindIr::Unit,
                        Fields::Unnamed(_) => VariantKindIr::Tuple,
                        Fields::Named(_) => VariantKindIr::Struct,
                    };
                    VariantIr {
                        name: variant.ident.clone(),
                        index,
                        kind,
                        fields: convert_fields(&variant.fields, &mut errors),
                        attributes,
                        discriminant: variant
                            .discriminant
                            .as_ref()
                            .map(|(_, expression)| expression.to_token_stream()),
                        span: variant.ident.span(),
                    }
                })
                .collect(),
            retained_tokens,
            span: input.ident.span(),
        },
        Data::Union(data) => TypeDeclarationIr {
            name: input.ident.clone(),
            kind: TypeDeclarationKindIr::Union,
            visibility,
            generics,
            attributes,
            fields: convert_fields(&Fields::Named(data.fields.clone()), &mut errors),
            variants: Vec::new(),
            retained_tokens,
            span: input.ident.span(),
        },
    };
    errors.finish(ParsedDeclaration {
        declaration: DeclarationIr::Type(declaration),
    })
}

/// Parses a `#[reflect]` trait input and removes nested helpers from retained tokens.
fn parse_trait(args: TokenStream, input: TokenStream) -> syn::Result<ParsedDeclaration> {
    let item: Item = syn::parse2(input)?;
    let Item::Trait(mut item) = item else {
        return Err(syn::Error::new(
            item.span(),
            "`#[reflect]` can only be applied to a trait",
        ));
    };
    let mut errors = ErrorCollector::default();
    let attributes = parse_helper_tokens(args, HelperTarget::Trait, &mut errors);
    let external_traits = attributes
        .iter()
        .filter_map(|attribute| match &attribute.value {
            crate::ir::HelperValueIr::ExternalTrait(value) => Some(value.clone()),
            _ => None,
        })
        .collect();
    let (methods, associated_types, associated_consts) = convert_trait_items(&item, &mut errors);
    strip_trait_helpers(&mut item);
    errors.finish(ParsedDeclaration {
        declaration: DeclarationIr::Trait(TraitDeclarationIr {
            name: item.ident.clone(),
            visibility: convert_visibility(&item.vis),
            generics: convert_generics(&item.generics),
            supertraits: item
                .supertraits
                .iter()
                .map(ToTokens::to_token_stream)
                .collect(),
            attributes,
            external_traits,
            methods,
            associated_types,
            associated_consts,
            retained_tokens: quote!(#item),
            span: item.ident.span(),
        }),
    })
}

/// Parses a `#[reflect_impl]` impl input and removes nested helpers from retained tokens.
fn parse_impl(args: TokenStream, input: TokenStream) -> syn::Result<ParsedDeclaration> {
    let item: Item = syn::parse2(input)?;
    let Item::Impl(mut item) = item else {
        return Err(syn::Error::new(
            item.span(),
            "`#[reflect_impl]` can only be applied to an impl block",
        ));
    };
    let mut errors = ErrorCollector::default();
    let attributes = parse_helper_tokens(args, HelperTarget::Impl, &mut errors);
    let specializations = attributes
        .iter()
        .filter_map(HelperAttributeIr::specialization)
        .cloned()
        .collect();
    let (methods, associated_types, associated_consts) = convert_impl_items(&item, &mut errors);
    let trait_path = item.trait_.as_ref().map(|(_, path, _)| convert_path(path));
    let target_type = convert_type(&item.self_ty);
    strip_impl_helpers(&mut item);
    errors.finish(ParsedDeclaration {
        declaration: DeclarationIr::Impl(ImplDeclarationIr {
            generics: convert_generics(&item.generics),
            target_type,
            trait_path,
            attributes,
            specializations,
            methods,
            associated_types,
            associated_consts,
            retained_tokens: quote!(#item),
            span: item.impl_token.span(),
        }),
    })
}

/// Converts a field collection in source order.
fn convert_fields(fields: &Fields, errors: &mut ErrorCollector) -> Vec<FieldIr> {
    fields
        .iter()
        .enumerate()
        .map(|(index, field)| FieldIr {
            name: field.ident.clone(),
            index,
            visibility: convert_visibility(&field.vis),
            ty: convert_type(&field.ty),
            attributes: parse_attributes(&field.attrs, HelperTarget::Field, errors),
            span: field.span(),
        })
        .collect()
}

/// Converts source generics without retaining a `syn::Generics` value.
fn convert_generics(generics: &Generics) -> GenericsIr {
    GenericsIr {
        params: generics
            .params
            .iter()
            .map(|parameter| match parameter {
                GenericParam::Lifetime(lifetime) => GenericParamIr {
                    name: lifetime.lifetime.ident.to_string(),
                    kind: GenericKindIr::Lifetime,
                    declaration: lifetime.to_token_stream(),
                    span: lifetime.span(),
                },
                GenericParam::Type(ty) => GenericParamIr {
                    name: ty.ident.to_string(),
                    kind: GenericKindIr::Type,
                    declaration: ty.to_token_stream(),
                    span: ty.span(),
                },
                GenericParam::Const(constant) => GenericParamIr {
                    name: constant.ident.to_string(),
                    kind: GenericKindIr::Const,
                    declaration: constant.to_token_stream(),
                    span: constant.span(),
                },
            })
            .collect(),
        where_predicates: generics
            .where_clause
            .iter()
            .flat_map(|clause| clause.predicates.iter())
            .map(ToTokens::to_token_stream)
            .collect(),
        declaration: generics.to_token_stream(),
    }
}

/// Normalizes a Rust visibility while preserving restricted paths.
fn convert_visibility(visibility: &Visibility) -> VisibilityIr {
    match visibility {
        Visibility::Public(_) => VisibilityIr::Public,
        Visibility::Inherited => VisibilityIr::Inherited,
        Visibility::Restricted(restricted) if restricted.path.is_ident("crate") => {
            VisibilityIr::Crate
        }
        Visibility::Restricted(restricted) if restricted.path.is_ident("super") => {
            VisibilityIr::Super
        }
        Visibility::Restricted(restricted) if restricted.path.is_ident("self") => {
            VisibilityIr::SelfValue
        }
        Visibility::Restricted(restricted) => {
            VisibilityIr::Restricted(convert_path(&restricted.path))
        }
    }
}

/// Converts all trait items relevant to later expansion.
fn convert_trait_items(
    item: &ItemTrait,
    errors: &mut ErrorCollector,
) -> (Vec<MethodIr>, Vec<AssociatedTypeIr>, Vec<AssociatedConstIr>) {
    let mut methods = Vec::new();
    let mut associated_types = Vec::new();
    let mut associated_consts = Vec::new();
    for trait_item in &item.items {
        match trait_item {
            TraitItem::Fn(function) => methods.push(convert_method(
                &function.sig,
                &Visibility::Inherited,
                &function.attrs,
                errors,
            )),
            TraitItem::Type(ty) => associated_types.push(AssociatedTypeIr {
                name: ty.ident.clone(),
                value: ty.default.as_ref().map(|(_, ty)| convert_type(ty)),
                declaration: ty.to_token_stream(),
                attributes: parse_attributes(&ty.attrs, HelperTarget::AssociatedItem, errors),
                span: ty.ident.span(),
            }),
            TraitItem::Const(constant) => associated_consts.push(AssociatedConstIr {
                name: constant.ident.clone(),
                ty: convert_type(&constant.ty),
                value: constant
                    .default
                    .as_ref()
                    .map(|(_, expression)| expression.to_token_stream()),
                declaration: constant.to_token_stream(),
                attributes: parse_attributes(&constant.attrs, HelperTarget::AssociatedItem, errors),
                span: constant.ident.span(),
            }),
            _ => {}
        }
    }
    (methods, associated_types, associated_consts)
}

/// Converts all impl items relevant to later expansion.
fn convert_impl_items(
    item: &ItemImpl,
    errors: &mut ErrorCollector,
) -> (Vec<MethodIr>, Vec<AssociatedTypeIr>, Vec<AssociatedConstIr>) {
    let mut methods = Vec::new();
    let mut associated_types = Vec::new();
    let mut associated_consts = Vec::new();
    for impl_item in &item.items {
        match impl_item {
            ImplItem::Fn(function) => methods.push(convert_method(
                &function.sig,
                &function.vis,
                &function.attrs,
                errors,
            )),
            ImplItem::Type(ty) => associated_types.push(AssociatedTypeIr {
                name: ty.ident.clone(),
                value: Some(convert_type(&ty.ty)),
                declaration: ty.to_token_stream(),
                attributes: parse_attributes(&ty.attrs, HelperTarget::AssociatedItem, errors),
                span: ty.ident.span(),
            }),
            ImplItem::Const(constant) => associated_consts.push(AssociatedConstIr {
                name: constant.ident.clone(),
                ty: convert_type(&constant.ty),
                value: Some(constant.expr.to_token_stream()),
                declaration: constant.to_token_stream(),
                attributes: parse_attributes(&constant.attrs, HelperTarget::AssociatedItem, errors),
                span: constant.ident.span(),
            }),
            _ => {}
        }
    }
    (methods, associated_types, associated_consts)
}

/// Converts a method signature and all method helpers.
fn convert_method(
    signature: &syn::Signature,
    visibility: &Visibility,
    attributes: &[syn::Attribute],
    errors: &mut ErrorCollector,
) -> MethodIr {
    let helper_attributes = parse_attributes(attributes, HelperTarget::Method, errors);
    let specializations = helper_attributes
        .iter()
        .filter_map(HelperAttributeIr::specialization)
        .cloned()
        .collect();
    let mut receiver = None;
    let mut parameters = Vec::new();
    for argument in &signature.inputs {
        match argument {
            FnArg::Receiver(value) => receiver = Some(value.to_token_stream()),
            FnArg::Typed(value) => {
                let name = match value.pat.as_ref() {
                    Pat::Ident(identifier) => Some(identifier.ident.to_string()),
                    _ => None,
                };
                parameters.push((name, convert_type(&value.ty), value.pat.to_token_stream()));
            }
        }
    }
    let return_type = match &signature.output {
        ReturnType::Default => ReturnTypeIr::Unit,
        ReturnType::Type(_, ty) => ReturnTypeIr::Type(convert_type(ty)),
    };
    let constness = &signature.constness;
    let asyncness = &signature.asyncness;
    let unsafety = &signature.unsafety;
    let abi = &signature.abi;
    let qualifiers = quote!(#constness #asyncness #unsafety #abi);
    MethodIr {
        name: signature.ident.clone(),
        visibility: convert_visibility(visibility),
        generics: convert_generics(&signature.generics),
        receiver,
        parameters,
        return_type,
        qualifiers,
        attributes: helper_attributes,
        specializations,
        span: signature.ident.span(),
    }
}

/// Removes nested helper attributes from a retained trait token stream.
fn strip_trait_helpers(item: &mut ItemTrait) {
    remove_reflect_attributes(&mut item.attrs);
    for trait_item in &mut item.items {
        match trait_item {
            TraitItem::Fn(function) => remove_reflect_attributes(&mut function.attrs),
            TraitItem::Type(ty) => remove_reflect_attributes(&mut ty.attrs),
            TraitItem::Const(constant) => remove_reflect_attributes(&mut constant.attrs),
            _ => {}
        }
    }
}

/// Removes nested helper attributes from a retained impl token stream.
fn strip_impl_helpers(item: &mut ItemImpl) {
    remove_reflect_attributes(&mut item.attrs);
    for impl_item in &mut item.items {
        match impl_item {
            ImplItem::Fn(function) => remove_reflect_attributes(&mut function.attrs),
            ImplItem::Type(ty) => remove_reflect_attributes(&mut ty.attrs),
            ImplItem::Const(constant) => remove_reflect_attributes(&mut constant.attrs),
            _ => {}
        }
    }
}
