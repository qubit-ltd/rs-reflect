//! Parsing of derive, trait, and impl declarations.

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::spanned::Spanned;
use syn::{
    Data, DeriveInput, Fields, FnArg, GenericParam, Generics, ImplItem, Item, ItemImpl, ItemTrait,
    Pat, ReturnType, TraitItem, Visibility,
};

use crate::ir::{
    AssociatedConstIr, AssociatedTypeIr, DeclarationIr, FieldIr, GenericBoundIr, GenericDefaultIr,
    GenericKindIr, GenericParamIr, GenericsIr, HelperAttributeIr, HelperTarget, ImplDeclarationIr,
    MacroKind, MethodIr, MethodQualifiersIr, ParameterIr, ParameterPatternIr,
    ParameterPatternKindIr, ParsedDeclaration, ReceiverIr, ReceiverKindIr, ReturnTypeIr,
    TraitDeclarationIr, TypeDeclarationIr, TypeDeclarationKindIr, ValidatedDeclaration, VariantIr,
    VariantKindIr, VisibilityIr, WherePredicateIr,
};
use crate::parse::attributes::{
    ErrorCollector, parse_attributes, parse_helper_tokens, remove_reflect_attributes,
};
use crate::parse::type_ir::{convert_bound, convert_path, convert_type};
use crate::validate::validation_error;

/// Parses a procedural macro invocation into shared declaration IR.
///
/// Returns combined syntax diagnostics when the input target or helper grammar is invalid.
#[allow(
    dead_code,
    reason = "the staged parse API is exercised directly by unit tests and later expansion tasks"
)]
pub(crate) fn parse_declaration(
    kind: MacroKind,
    args: TokenStream,
    input: TokenStream,
) -> syn::Result<ParsedDeclaration> {
    let pipeline = parse_pipeline(kind, args, input)?;
    match pipeline.error {
        Some(error) => Err(error),
        None => Ok(pipeline.declaration),
    }
}

/// Parses and validates one macro invocation while aggregating recoverable diagnostics.
pub(crate) fn parse_and_validate_declaration(
    kind: MacroKind,
    args: TokenStream,
    input: TokenStream,
) -> syn::Result<ValidatedDeclaration> {
    let pipeline = parse_pipeline(kind, args, input)?;
    let mut combined = pipeline.error;
    if let Some(validation) = validation_error(&pipeline.declaration.declaration) {
        if let Some(error) = &mut combined {
            error.combine(validation);
        } else {
            combined = Some(validation);
        }
    }
    match combined {
        Some(error) => Err(error),
        None => Ok(ValidatedDeclaration {
            declaration: pipeline.declaration.declaration,
        }),
    }
}

/// Holds parser diagnostics alongside IR until the validation pipeline completes.
struct ParsedPipeline {
    declaration: ParsedDeclaration,
    error: Option<syn::Error>,
}

/// Runs only the syntax and IR conversion phase for one macro kind.
fn parse_pipeline(
    kind: MacroKind,
    args: TokenStream,
    input: TokenStream,
) -> syn::Result<ParsedPipeline> {
    match kind {
        MacroKind::Derive => parse_derive(args, input),
        MacroKind::Trait => parse_trait(args, input),
        MacroKind::Impl => parse_impl(args, input),
    }
}

/// Parses a `Reflect` derive input.
fn parse_derive(args: TokenStream, input: TokenStream) -> syn::Result<ParsedPipeline> {
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
    Ok(ParsedPipeline {
        declaration: ParsedDeclaration {
            declaration: DeclarationIr::Type(declaration),
        },
        error: errors.into_error(),
    })
}

/// Parses a `#[reflect]` trait input and removes nested helpers from retained tokens.
fn parse_trait(args: TokenStream, input: TokenStream) -> syn::Result<ParsedPipeline> {
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
    Ok(ParsedPipeline {
        declaration: ParsedDeclaration {
            declaration: DeclarationIr::Trait(TraitDeclarationIr {
                name: item.ident.clone(),
                visibility: convert_visibility(&item.vis),
                generics: convert_generics(&item.generics),
                supertraits: item.supertraits.iter().map(convert_bound).collect(),
                attributes,
                external_traits,
                methods,
                associated_types,
                associated_consts,
                retained_tokens: quote!(#item),
                span: item.ident.span(),
            }),
        },
        error: errors.into_error(),
    })
}

/// Parses a `#[reflect_impl]` impl input and removes nested helpers from retained tokens.
fn parse_impl(args: TokenStream, input: TokenStream) -> syn::Result<ParsedPipeline> {
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
    Ok(ParsedPipeline {
        declaration: ParsedDeclaration {
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
        },
        error: errors.into_error(),
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
    let (impl_declaration, arguments, where_clause) = generics.split_for_impl();
    GenericsIr {
        params: generics
            .params
            .iter()
            .map(|parameter| match parameter {
                GenericParam::Lifetime(lifetime) => GenericParamIr {
                    name: lifetime.lifetime.ident.to_string(),
                    kind: GenericKindIr::Lifetime,
                    bounds: lifetime
                        .bounds
                        .iter()
                        .map(|bound| GenericBoundIr::Lifetime(bound.to_token_stream().to_string()))
                        .collect(),
                    default: None,
                    const_type: None,
                    declaration: lifetime.to_token_stream(),
                    span: lifetime.span(),
                },
                GenericParam::Type(ty) => GenericParamIr {
                    name: ty.ident.to_string(),
                    kind: GenericKindIr::Type,
                    bounds: ty.bounds.iter().map(convert_bound).collect(),
                    default: ty
                        .default
                        .as_ref()
                        .map(convert_type)
                        .map(GenericDefaultIr::Type),
                    const_type: None,
                    declaration: ty.to_token_stream(),
                    span: ty.span(),
                },
                GenericParam::Const(constant) => GenericParamIr {
                    name: constant.ident.to_string(),
                    kind: GenericKindIr::Const,
                    bounds: Vec::new(),
                    default: constant
                        .default
                        .as_ref()
                        .map(ToTokens::to_token_stream)
                        .map(GenericDefaultIr::Const),
                    const_type: Some(convert_type(&constant.ty)),
                    declaration: constant.to_token_stream(),
                    span: constant.span(),
                },
            })
            .collect(),
        where_predicates: generics
            .where_clause
            .iter()
            .flat_map(|clause| clause.predicates.iter())
            .map(|predicate| match predicate {
                syn::WherePredicate::Lifetime(lifetime) => WherePredicateIr::Lifetime {
                    lifetime: lifetime.lifetime.to_token_stream().to_string(),
                    bounds: lifetime
                        .bounds
                        .iter()
                        .map(ToTokens::to_token_stream)
                        .map(|value| value.to_string())
                        .collect(),
                    declaration: lifetime.to_token_stream(),
                },
                syn::WherePredicate::Type(ty) => WherePredicateIr::Type {
                    bounded_type: convert_type(&ty.bounded_ty),
                    lifetimes: ty
                        .lifetimes
                        .iter()
                        .flat_map(|lifetimes| lifetimes.lifetimes.iter())
                        .filter_map(|parameter| match parameter {
                            GenericParam::Lifetime(lifetime) => {
                                Some(lifetime.lifetime.to_token_stream().to_string())
                            }
                            _ => None,
                        })
                        .collect(),
                    bounds: ty.bounds.iter().map(convert_bound).collect(),
                    declaration: ty.to_token_stream(),
                },
                _ => WherePredicateIr::Other(predicate.to_token_stream()),
            })
            .collect(),
        declaration: generics.to_token_stream(),
        impl_declaration: impl_declaration.to_token_stream(),
        arguments: arguments.to_token_stream(),
        where_clause: where_clause.to_token_stream(),
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
                function.default.is_some(),
                errors,
            )),
            TraitItem::Type(ty) => associated_types.push(AssociatedTypeIr {
                name: ty.ident.clone(),
                generics: convert_generics(&ty.generics),
                bounds: ty.bounds.iter().map(convert_bound).collect(),
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
                true,
                errors,
            )),
            ImplItem::Type(ty) => associated_types.push(AssociatedTypeIr {
                name: ty.ident.clone(),
                generics: convert_generics(&ty.generics),
                bounds: Vec::new(),
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
    has_default: bool,
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
            FnArg::Receiver(value) => {
                let kind = if value.colon_token.is_some() {
                    ReceiverKindIr::Typed
                } else if value.reference.is_some() && value.mutability.is_some() {
                    ReceiverKindIr::MutableReference
                } else if value.reference.is_some() {
                    ReceiverKindIr::SharedReference
                } else {
                    ReceiverKindIr::Value
                };
                receiver = Some(ReceiverIr {
                    kind,
                    ty: convert_type(&value.ty),
                    declaration: value.to_token_stream(),
                    span: value.span(),
                });
            }
            FnArg::Typed(value) => {
                let pattern_tokens = value.pat.to_token_stream();
                let (name, kind) = match value.pat.as_ref() {
                    Pat::Ident(identifier) => (
                        Some(identifier.ident.to_string()),
                        ParameterPatternKindIr::Identifier,
                    ),
                    Pat::Wild(_) => (None, ParameterPatternKindIr::Wildcard),
                    _ => (None, ParameterPatternKindIr::Destructure),
                };
                parameters.push(ParameterIr {
                    name,
                    pattern: ParameterPatternIr {
                        kind,
                        source: pattern_tokens.to_string(),
                        tokens: pattern_tokens,
                    },
                    ty: convert_type(&value.ty),
                    index: parameters.len(),
                    span: value.span(),
                });
            }
        }
    }
    let return_type = match &signature.output {
        ReturnType::Default => ReturnTypeIr::Unit,
        ReturnType::Type(_, ty) if matches!(ty.as_ref(), syn::Type::Tuple(tuple) if tuple.elems.is_empty()) => {
            ReturnTypeIr::Unit
        }
        ReturnType::Type(_, ty) => ReturnTypeIr::Type(convert_type(ty)),
    };
    let qualifiers = MethodQualifiersIr {
        is_const: signature.constness.is_some(),
        is_async: signature.asyncness.is_some(),
        is_unsafe: signature.unsafety.is_some(),
        abi: signature.abi.as_ref().map(|abi| {
            abi.name
                .as_ref()
                .map_or_else(|| "C".to_owned(), syn::LitStr::value)
        }),
        is_variadic: signature.variadic.is_some(),
    };
    MethodIr {
        name: signature.ident.clone(),
        visibility: convert_visibility(visibility),
        generics: convert_generics(&signature.generics),
        receiver,
        parameters,
        return_type,
        qualifiers,
        has_default,
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
