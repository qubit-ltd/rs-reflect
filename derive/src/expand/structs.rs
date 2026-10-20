// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Expansion of reflected struct declarations.

use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;

use crate::expand::ExpansionContext;
use crate::expand::generic_environment::GenericEnvironment;
use crate::ir::GenericKindIr;
use crate::ir::HelperName;
use crate::ir::TypeDeclarationIr;
use crate::ir::TypeDeclarationKindIr;
use crate::ir::VisibilityIr;

/// Expands a concrete struct into a static root descriptor and safe field
/// adapters.
///
/// # Parameters
///
/// - `declaration`: Validated struct declaration and its retained source facts.
/// - `context`: Expansion configuration and runtime facade resolver.
///
/// # Returns
///
/// Returns generated items or a diagnostic if retained generic syntax is
/// malformed.
///
/// # Errors
///
/// Returns a syntax diagnostic when the declaration is not a struct or retained
/// generic syntax cannot be parsed.
pub(crate) fn expand(declaration: TypeDeclarationIr, context: &ExpansionContext) -> syn::Result<TokenStream> {
    expand_v4(declaration, context)
}

fn expand_v4(declaration: TypeDeclarationIr, context: &ExpansionContext) -> syn::Result<TokenStream> {
    if declaration.kind != TypeDeclarationKindIr::Struct {
        return Err(syn::Error::new(
            declaration.span,
            "cannot expand Reflect for non-struct declaration",
        ));
    }
    let facade = context.facade().clone();
    let name = declaration.name.clone();
    let fingerprint = context.fingerprint(&declaration.retained_tokens.to_string());
    let module = format_ident!("__qubit_reflect_v4_struct_{fingerprint:016x}");
    let marker = format_ident!("TypeDefinitionMarker");
    let owner_name = declaration.name.to_string();
    let mut generics = super::generics::parse_type_generics(&declaration)?;
    let visible_fields: Vec<_> = declaration
        .fields
        .iter()
        .filter(|field| {
            !field
                .attributes
                .iter()
                .any(|attribute| attribute.name == HelperName::Skip)
        })
        .collect();
    let environment = GenericEnvironment::from_generics(&declaration.generics);
    let constructible = visible_fields
        .iter()
        .all(|field| supports_construction(&field.ty, &owner_name))
        && !has_skipped_source_field(&declaration)
        && declaration
            .fields
            .iter()
            .all(|field| super::generics::field_cfg_attributes(&declaration, None, field.index).is_empty())
        && visible_fields.len() == declaration.fields.len();
    {
        let where_clause = generics.make_where_clause();
        for parameter in declaration
            .generics
            .params
            .iter()
            .filter(|parameter| parameter.kind == GenericKindIr::Lifetime)
        {
            let lifetime = syn::Lifetime::new(&format!("'{}", parameter.name), parameter.span);
            where_clause.predicates.push(syn::parse_quote!(#lifetime: 'static));
        }
        for parameter in declaration
            .generics
            .params
            .iter()
            .filter(|parameter| parameter.kind == GenericKindIr::Type)
        {
            let parameter = syn::Ident::new(&parameter.name, parameter.span);
            where_clause.predicates.push(syn::parse_quote!(#parameter: 'static));
        }
        for field in visible_fields.iter().filter(|field| {
            supports_reflection(&field.ty, &owner_name)
                && super::generics::field_cfg_attributes(&declaration, None, field.index).is_empty()
        }) {
            let ty = &field.ty.tokens;
            where_clause
                .predicates
                .push(syn::parse_quote!(#ty: #facade::__private::codegen_v4::descriptor::Reflect));
        }
    }
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let mut field_functions = Vec::new();
    let mut field_metadata = Vec::new();
    let mut construct_fields = Vec::new();
    for field in &visible_fields {
        let cfg = super::generics::field_cfg_attributes(&declaration, None, field.index);
        let index = field.index;
        let get = format_ident!("__qubit_reflect_v4_get_field_{fingerprint:016x}_{index}");
        let get_mut = format_ident!("__qubit_reflect_v4_get_mut_field_{fingerprint:016x}_{index}");
        let ty = &field.ty.tokens;
        let member = field
            .name
            .as_ref()
            .map(|field_name| quote!(#field_name))
            .unwrap_or_else(|| {
                let index = syn::Index::from(index);
                quote!(#index)
            });
        let source_name = field
            .name
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| index.to_string());
        let visibility = v4_visibility(&field.visibility, &facade, field.span);
        let expression = super::expression_codegen::type_expression(&field.ty, &environment, &facade);
        field_functions.push(quote! {
            #(#cfg)*
            fn #get<'a>(value: &'a Self) -> &'a #ty { &value.#member }
            #(#cfg)*
            fn #get_mut<'a>(value: &'a mut Self) -> &'a mut #ty { &mut value.#member }
        });
        let mutable = true;
        let access = quote!(#facade::__private::codegen_v4::access::FieldAccess::new(
            true, #mutable, #mutable, true,
        ));
        let adapter = quote!(::std::boxed::Box::leak(::std::boxed::Box::new(
            #facade::__private::codegen_v4::access::TypedFieldAdapter::<Self, #ty>::new(Self::#get, Self::#get_mut)
        )));
        let local_dynamic = if supports_construction(&field.ty, &owner_name) && cfg.is_empty() {
            quote!(.with_local_dynamic(#adapter))
        } else {
            TokenStream::new()
        };
        let thread_safe_dynamic = if constructible && declaration.generics.params.is_empty() {
            quote!(.with_thread_safe_dynamic(#adapter))
        } else {
            TokenStream::new()
        };
        let concrete_type = if supports_reflection(&field.ty, &owner_name) && cfg.is_empty() {
            quote!(.with_concrete_type::<#ty>())
        } else {
            TokenStream::new()
        };
        field_metadata.push(quote! {
            #(#cfg)*
            #facade::__private::codegen_v4::access::FieldMetadata::typed::<Self, #ty>(
                #index, #source_name, #visibility, #expression,
                #adapter, #access,
            ) #concrete_type #local_dynamic #thread_safe_dynamic
        });
        let field_policy = if constructible {
            quote!(#facade::__private::codegen_v4::construct::ConstructionFieldPolicy::Required)
        } else {
            quote!(#facade::__private::codegen_v4::construct::ConstructionFieldPolicy::Unavailable(
                #facade::__private::codegen_v4::construct::ConstructionUnavailableReason::FieldNotReflectable,
            ))
        };
        let update_policy = if constructible {
            quote!(#facade::__private::codegen_v4::construct::UpdateFieldPolicy::Replaceable)
        } else {
            quote!(#facade::__private::codegen_v4::construct::UpdateFieldPolicy::Unavailable(
                #facade::__private::codegen_v4::construct::UpdateUnavailableReason::FieldNotReplaceable,
            ))
        };
        construct_fields.push(quote! {
            #(#cfg)*
            #facade::__private::codegen_v4::construct::ConstructField::typed::<#ty>(
                #source_name, #visibility, #field_policy, #update_policy,
            )
        });
    }
    let (provider_functions, providers, availability) =
        struct_provider(&declaration, &facade, constructible, fingerprint);
    field_functions.push(provider_functions);
    let struct_kind = match declaration.field_shape {
        crate::ir::FieldShapeIr::Unit => {
            quote!(#facade::__private::codegen_v4::expression::StructKind::Unit)
        }
        crate::ir::FieldShapeIr::Named => {
            quote!(#facade::__private::codegen_v4::expression::StructKind::Named)
        }
        crate::ir::FieldShapeIr::Unnamed => {
            quote!(#facade::__private::codegen_v4::expression::StructKind::Tuple)
        }
    };
    let kind = quote!(#facade::__private::codegen_v4::expression::TypeDefinitionKind::Struct(#struct_kind));
    let type_definition_ident = format_ident!("__qubit_reflect_type_definition");
    let generic_definition =
        super::expression_codegen::generic_definition(&declaration.generics, declaration.span, &facade);
    let generic_arguments =
        super::expression_codegen::generic_arguments(&declaration.generics, &type_definition_ident, &facade);
    let type_definition_and_arguments = {
        let rust_name = name.to_string();
        quote!({
            let generic_definition = ::std::boxed::Box::leak(::std::boxed::Box::new(#generic_definition));
            let #type_definition_ident: &'static #facade::__private::codegen_v4::expression::TypeDefinitionMetadata = ::std::boxed::Box::leak(::std::boxed::Box::new(
                #facade::__private::codegen_v4::expression::TypeDefinitionMetadata::new::<#module::#marker>(
                    #rust_name,
                    concat!(module_path!(), "::", stringify!(#name)),
                    #kind,
                    generic_definition,
                ),
            ));
            let generic_arguments: &'static [#facade::__private::codegen_v4::expression::GenericArgumentMetadata] = #generic_arguments;
            (Some(#type_definition_ident), generic_arguments)
        })
    };
    let (shape, construction_shape) = match declaration.field_shape {
        crate::ir::FieldShapeIr::Unit => (
            quote!(#facade::__private::codegen_v4::access::TypeKindMetadata::Struct),
            quote!(#facade::__private::codegen_v4::construct::ConstructionShape::Unit),
        ),
        crate::ir::FieldShapeIr::Named => (
            quote!(#facade::__private::codegen_v4::access::TypeKindMetadata::Struct),
            quote!(#facade::__private::codegen_v4::construct::ConstructionShape::Named),
        ),
        crate::ir::FieldShapeIr::Unnamed => (
            quote!(#facade::__private::codegen_v4::access::TypeKindMetadata::Tuple),
            quote!(#facade::__private::codegen_v4::construct::ConstructionShape::Positional),
        ),
    };
    let plans = quote!({
        let construct_fields = ::std::boxed::Box::leak(::std::vec![#(#construct_fields),*].into_boxed_slice());
        let plan = #facade::__private::codegen_v4::construct::ConstructionPlan::for_type::<Self>(
            #construction_shape,
            construct_fields,
            #providers,
            #availability,
        );
        ::std::boxed::Box::leak(::std::vec![plan].into_boxed_slice())
    });
    let shape_expr = quote!({
        let fields = ::std::boxed::Box::leak(::std::vec![#(#field_metadata),*].into_boxed_slice());
        ::std::boxed::Box::leak(::std::boxed::Box::new(#shape(fields)))
    });
    let type_submission = if declaration.generics.params.is_empty() {
        quote!(#facade::__private::codegen_v4::registration::submit_type!(#name);)
    } else {
        let registration_fn = format_ident!("{}_registration", module);
        let definition_fn = format_ident!("{}_definition_registration", module);
        let capability_fn = format_ident!("__qubit_reflect_v4_definition_capability_{}", name);
        let rust_name = name.to_string();
        quote! {
            #[doc(hidden)]
            fn #definition_fn() -> &'static #facade::__private::codegen_v4::registration::TypeDefinitionRegistration<#module::#marker> {
                static VALUE: ::std::sync::OnceLock<
                    ::std::boxed::Box<#facade::__private::codegen_v4::registration::TypeDefinitionRegistration<#module::#marker>>,
                > = ::std::sync::OnceLock::new();
                VALUE.get_or_init(|| {
                    let generics = ::std::boxed::Box::leak(::std::boxed::Box::new(#generic_definition));
                    ::std::boxed::Box::new(
                        #facade::__private::codegen_v4::registration::TypeDefinitionRegistration::new(
                            #rust_name,
                            concat!(module_path!(), "::", stringify!(#name)),
                            #kind,
                            generics,
                        ),
                    )
                })
            }
            #[doc(hidden)]
            fn #registration_fn() -> #facade::__private::codegen_v4::registration::Registration {
                #facade::__private::codegen_v4::registration::Registration::type_definition(#definition_fn())
            }
            #[doc(hidden)]
            #[allow(non_snake_case)]
            pub fn #capability_fn<C: #facade::capability::CapabilitySpec>(adapter: C::Adapter) -> #facade::capability::CapabilityRegistration {
                #facade::capability::CapabilityRegistration::for_type_definition::<#module::#marker, C>(adapter)
            }
            #facade::__private::codegen_v4::registration::submit_type_definition!(#rust_name, #registration_fn);
        }
    };
    Ok(quote! {
        #[allow(non_camel_case_types)]
        mod #module { pub struct #marker; }
        impl #impl_generics #name #type_generics #where_clause {
            #(#field_functions)*
        }
        impl #impl_generics #facade::__private::codegen_v4::descriptor::Reflect for #name #type_generics #where_clause {
            fn __reflect_registration() -> &'static #facade::__private::codegen_v4::descriptor::TypeRegistration<Self> {
                #facade::__private::codegen_v4::descriptor::TypeRegistration::<Self>::get_or_init_with(|| {
                    let (type_definition, generic_arguments) = #type_definition_and_arguments;
                    #facade::__private::codegen_v4::descriptor::TypeRegistration::<Self>::with_shape(
                        #shape_expr, type_definition, #plans,
                    ).with_generic_arguments(generic_arguments)
                })
            }
        }
        #type_submission
    })
}

pub(super) fn supports_construction(ty: &crate::ir::TypeIr, owner_name: &str) -> bool {
    use crate::ir::TypeKindIr;
    if contains_recursive_owner_type(ty, owner_name) {
        return false;
    }
    match &ty.kind {
        TypeKindIr::Path(path) if path.qualified_self.is_none() => !path.segments.is_empty(),
        TypeKindIr::Tuple(values) => values.iter().all(|value| supports_construction(value, owner_name)),
        TypeKindIr::Array { element, .. } => supports_construction(element, owner_name),
        _ => false,
    }
}

pub(super) fn supports_reflection(ty: &crate::ir::TypeIr, owner_name: &str) -> bool {
    use crate::ir::TypeKindIr;

    if contains_recursive_owner_type(ty, owner_name) {
        return false;
    }
    match &ty.kind {
        TypeKindIr::Path(path) if path.qualified_self.is_none() => !path.segments.is_empty(),
        TypeKindIr::Tuple(values) => values.iter().all(|value| supports_reflection(value, owner_name)),
        TypeKindIr::Array { element, .. } => supports_reflection(element, owner_name),
        _ => false,
    }
}

fn contains_recursive_owner_type(ty: &crate::ir::TypeIr, owner_name: &str) -> bool {
    use crate::ir::PathArgumentIr;
    use crate::ir::PathArgumentsIr;
    use crate::ir::TypeKindIr;

    match &ty.kind {
        TypeKindIr::Path(path) => {
            path.qualified_self
                .as_ref()
                .is_some_and(|qualified| contains_recursive_owner_type(&qualified.ty, owner_name))
                || path
                    .segments
                    .last()
                    .is_some_and(|segment| segment.name == owner_name || segment.name == "Self")
                || path.segments.iter().any(|segment| match &segment.arguments {
                    PathArgumentsIr::AngleBracketed(arguments) => arguments.iter().any(|argument| match argument {
                        PathArgumentIr::Type(nested) | PathArgumentIr::AssociatedType { ty: nested, .. } => {
                            contains_recursive_owner_type(nested, owner_name)
                        }
                        _ => false,
                    }),
                    PathArgumentsIr::Parenthesized { inputs, output } => {
                        inputs
                            .iter()
                            .any(|input| contains_recursive_owner_type(input, owner_name))
                            || output
                                .as_ref()
                                .is_some_and(|output| contains_recursive_owner_type(output, owner_name))
                    }
                    PathArgumentsIr::None => false,
                })
        }
        TypeKindIr::Reference { element, .. }
        | TypeKindIr::Slice(element)
        | TypeKindIr::Array { element, .. }
        | TypeKindIr::Pointer { element, .. } => contains_recursive_owner_type(element, owner_name),
        TypeKindIr::Tuple(elements) => elements
            .iter()
            .any(|element| contains_recursive_owner_type(element, owner_name)),
        TypeKindIr::BareFunction { inputs, output, .. } => {
            inputs
                .iter()
                .any(|input| contains_recursive_owner_type(input, owner_name))
                || output
                    .as_ref()
                    .is_some_and(|output| contains_recursive_owner_type(output, owner_name))
        }
        _ => false,
    }
}

fn has_skipped_source_field(declaration: &TypeDeclarationIr) -> bool {
    let input: syn::DeriveInput =
        syn::parse2(declaration.retained_tokens.clone()).expect("the parser retains a valid struct declaration");
    let syn::Data::Struct(data) = input.data else {
        return false;
    };
    data.fields.iter().any(|field| {
        field.attrs.iter().any(|attribute| {
            attribute.path().is_ident("reflect")
                && attribute
                    .parse_args_with(syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated)
                    .is_ok_and(|items| items.iter().any(|item| item.path().is_ident("skip")))
        })
    })
}

fn struct_provider(
    declaration: &TypeDeclarationIr,
    facade: &TokenStream,
    constructible: bool,
    fingerprint: u64,
) -> (TokenStream, TokenStream, TokenStream) {
    if !constructible {
        return (
            TokenStream::new(),
            quote!(#facade::__private::codegen_v4::construct::ConstructionProviders::new::<Self>(None)),
            quote!(#facade::__private::codegen_v4::construct::ConstructionAvailability::Unavailable {
                reasons: ::std::vec![#facade::__private::codegen_v4::construct::ConstructionUnavailableReason::FieldNotReflectable].into_boxed_slice(),
            }),
        );
    }
    let fields: Vec<_> = declaration
        .fields
        .iter()
        .filter(|field| {
            !field
                .attributes
                .iter()
                .any(|attribute| attribute.name == HelperName::Skip)
        })
        .collect();
    let (local_functions, local_provider) =
        struct_provider_mode(declaration, facade, &fields, "local", quote!(Local), fingerprint);
    let (thread_safe_functions, thread_safe_provider) = if declaration.generics.params.is_empty() {
        struct_provider_mode(
            declaration,
            facade,
            &fields,
            "thread_safe",
            quote!(ThreadSafe),
            fingerprint,
        )
    } else {
        (TokenStream::new(), TokenStream::new())
    };
    let make_functions = quote!(#local_functions #thread_safe_functions);
    let local_providers =
        quote!(#facade::__private::codegen_v4::construct::ConstructionProviders::new::<Self>(Some(#local_provider)));
    let provider = if declaration.generics.params.is_empty() {
        quote!({
            let providers = #local_providers;
            providers.with_thread_safe::<Self>(#thread_safe_provider)
        })
    } else {
        local_providers
    };
    (
        make_functions,
        provider,
        quote!(#facade::__private::codegen_v4::construct::ConstructionAvailability::Available),
    )
}

fn struct_provider_mode(
    declaration: &TypeDeclarationIr,
    facade: &TokenStream,
    fields: &[&crate::ir::FieldIr],
    suffix: &str,
    mode: TokenStream,
    fingerprint: u64,
) -> (TokenStream, TokenStream) {
    let construct_name = format_ident!("__qubit_reflect_v4_construct_{suffix}_{fingerprint:016x}");
    let update_name = format_ident!("__qubit_reflect_v4_update_{suffix}_{fingerprint:016x}");
    let mut construct_bindings = Vec::new();
    let mut initializers = Vec::new();
    let mut updates = Vec::new();
    for (position, field) in fields.iter().enumerate() {
        let variable = format_ident!("field_{position}");
        let ty = &field.ty.tokens;
        let index = syn::Index::from(position);
        construct_bindings.push(quote!(let #variable = fields.get_mut(#index).and_then(Option::take).expect("validated construction field must be present")
            .downcast::<#ty>().unwrap_or_else(|_| unreachable!("validated construction field type"));));
        let member = field
            .name
            .as_ref()
            .map(|name| quote!(#name))
            .unwrap_or_else(|| quote!(#index));
        if field.name.is_some() {
            initializers.push(quote!(#member: #variable));
        } else {
            initializers.push(quote!(#variable));
        }
        updates.push(quote! {
            if let Some(replacement) = fields.get_mut(#index).and_then(Option::take) {
                value.#member = replacement.downcast::<#ty>()
                    .unwrap_or_else(|_| unreachable!("validated update field type"));
            }
        });
    }
    let construct_value = match declaration.field_shape {
        crate::ir::FieldShapeIr::Unit => quote!(Self),
        crate::ir::FieldShapeIr::Named => quote!(Self { #(#initializers),* }),
        crate::ir::FieldShapeIr::Unnamed => quote!(Self(#(#initializers),*)),
    };
    let functions = quote! {
        fn #construct_name(mut fields: ::std::vec::Vec<Option<#facade::__private::codegen_v4::value::DynamicOwned<#facade::__private::codegen_v4::value::#mode>>>) -> Self {
            #(#construct_bindings)*
            #construct_value
        }
        fn #update_name(mut value: Self, mut fields: ::std::vec::Vec<Option<#facade::__private::codegen_v4::value::DynamicOwned<#facade::__private::codegen_v4::value::#mode>>>) -> Self {
            #(#updates)*
            value
        }
    };
    let provider = quote! {
        ::std::boxed::Box::leak(::std::boxed::Box::new(
            #facade::__private::codegen_v4::construct::ConstructProvider::<Self, #facade::__private::codegen_v4::value::#mode>::new(
                Self::#construct_name, Self::#update_name,
            )
        ))
    };
    (functions, provider)
}

pub(super) fn v4_visibility(visibility: &VisibilityIr, facade: &TokenStream, span: Span) -> TokenStream {
    match visibility {
        VisibilityIr::Public => quote!(#facade::__private::codegen_v4::access::Visibility::Public),
        VisibilityIr::Crate => quote!(#facade::__private::codegen_v4::access::Visibility::Crate),
        VisibilityIr::Super => {
            quote!(#facade::__private::codegen_v4::access::Visibility::Restricted("super"))
        }
        VisibilityIr::SelfValue | VisibilityIr::Inherited => {
            quote!(#facade::__private::codegen_v4::access::Visibility::Private)
        }
        VisibilityIr::Restricted(path) => {
            let path = syn::LitStr::new(&path.source, span);
            quote!(#facade::__private::codegen_v4::access::Visibility::Restricted(#path))
        }
    }
}

#[cfg(test)]
mod tests {
    use proc_macro2::TokenStream;
    use quote::quote;

    use super::expand;
    use crate::expand::ExpansionContext;
    use crate::ir::DeclarationIr;
    use crate::ir::MacroKind;
    use crate::parse::parse_and_validate_declaration;

    #[test]
    fn test_struct_emission_uses_codegen_v4_only() {
        let parsed = parse_and_validate_declaration(
            MacroKind::Derive,
            TokenStream::new(),
            quote! {
                struct Fixture<T> {
                    value: T,
                    unreflected: Unreflected,
                    #[reflect(skip)]
                    skipped: Hidden,
                }
            },
        )
        .expect("the structural fixture should parse");
        let DeclarationIr::Type(declaration) = parsed.declaration else {
            panic!("the structural fixture should produce a type declaration");
        };
        let generated = expand(declaration, &ExpansionContext::for_tests())
            .expect("the structural fixture should expand")
            .to_string();

        assert!(
            generated.contains("codegen_v4"),
            "struct tokens must use the v4 protocol"
        );
        assert!(
            !generated.contains(concat!("codegen_", "v3")),
            "struct tokens must not use the v3 protocol"
        );
        assert!(
            generated.contains("Unreflected"),
            "unreflected fields retain their type expression"
        );
        assert!(
            generated.contains("ExpressionData :: Named"),
            "unreflected fields retain TypeExpression metadata"
        );
        assert!(generated.contains("with_local_dynamic"));
        assert!(!generated.contains("with_thread_safe_dynamic"));
        assert!(!generated.contains("\"skipped\""), "skipped fields produce no metadata");
        for forbidden in [
            "TypeId",
            "TypeIdentity",
            "FragmentIdentity",
            "RuntimeIdentity",
            "inventory",
        ] {
            assert!(
                !generated.contains(forbidden),
                "generated tokens must not mention {forbidden}"
            );
        }
    }

    #[test]
    fn test_struct_construction_emits_typed_providers_for_named_tuple_and_unit() {
        for (source, has_fields) in [
            (
                quote!(
                    struct Named {
                        value: u8,
                    }
                ),
                true,
            ),
            (
                quote!(
                    struct Tuple(u8);
                ),
                true,
            ),
            (
                quote!(
                    struct Unit;
                ),
                false,
            ),
        ] {
            let parsed = parse_and_validate_declaration(MacroKind::Derive, TokenStream::new(), source)
                .expect("the struct fixture should parse");
            let DeclarationIr::Type(declaration) = parsed.declaration else {
                panic!("the fixture should be a struct");
            };
            let generated = expand(declaration, &ExpansionContext::for_tests())
                .expect("the fixture should expand")
                .to_string();
            assert!(generated.contains("ConstructProvider"));
            assert!(generated.contains("ConstructionAvailability :: Available"));
            assert_eq!(generated.contains("with_thread_safe_dynamic"), has_fields);
            assert!(!generated.contains(concat!("codegen_", "v3")));
        }
    }

    #[test]
    fn test_cfg_gated_fields_keep_metadata_aligned_without_construction_provider() {
        let parsed = parse_and_validate_declaration(
            MacroKind::Derive,
            TokenStream::new(),
            quote! {
                struct CfgFixture {
                    #[cfg(any())]
                    conditional: u8,
                    stable: u16,
                }
            },
        )
        .expect("the cfg fixture should parse");
        let DeclarationIr::Type(declaration) = parsed.declaration else {
            panic!("the cfg fixture should be a struct");
        };
        let generated = expand(declaration, &ExpansionContext::for_tests())
            .expect("the cfg fixture should expand")
            .to_string();

        assert!(generated.contains("cfg (any ())"));
        assert!(generated.contains("ConstructionAvailability :: Unavailable"));
        assert!(!generated.contains("ConstructProvider"));
        assert!(!generated.contains(concat!("codegen_", "v3")));
    }

    #[test]
    fn test_struct_helper_functions_are_associated_items() {
        let parsed = parse_and_validate_declaration(
            MacroKind::Derive,
            TokenStream::new(),
            quote! {
                struct Fixture {
                    value: u8,
                }
            },
        )
        .expect("the struct fixture should parse");
        let DeclarationIr::Type(declaration) = parsed.declaration else {
            panic!("the fixture should be a struct");
        };
        let generated = expand(declaration, &ExpansionContext::for_tests()).expect("the struct should expand");
        let file = syn::parse2::<syn::File>(generated).expect("generated struct tokens must parse");
        let implementation = file
            .items
            .iter()
            .find_map(|item| match item {
                syn::Item::Impl(implementation) if implementation.trait_.is_none() => Some(implementation),
                _ => None,
            })
            .expect("the expansion should contain an inherent helper impl");
        let method_names: Vec<_> = implementation
            .items
            .iter()
            .filter_map(|item| match item {
                syn::ImplItem::Fn(method) => Some(method.sig.ident.to_string()),
                _ => None,
            })
            .collect();

        for expected_prefix in [
            "__qubit_reflect_v4_get_field_",
            "__qubit_reflect_v4_get_mut_field_",
            "__qubit_reflect_v4_construct_local_",
            "__qubit_reflect_v4_update_local_",
        ] {
            assert!(
                method_names.iter().any(|name| name.starts_with(expected_prefix)),
                "inherent helper impl must declare `{expected_prefix}…`: {method_names:?}"
            );
        }
    }

    #[test]
    fn test_skipped_source_field_disables_incomplete_construction_provider() {
        let parsed = parse_and_validate_declaration(
            MacroKind::Derive,
            TokenStream::new(),
            quote! {
                struct Account {
                    id: u32,
                    #[reflect(skip)]
                    internal: String,
                }
            },
        )
        .expect("the account fixture should parse");
        let DeclarationIr::Type(declaration) = parsed.declaration else {
            panic!("the fixture should be a struct");
        };
        let generated = expand(declaration, &ExpansionContext::for_tests())
            .expect("the account should expand")
            .to_string();

        assert!(generated.contains("ConstructionAvailability :: Unavailable"));
        assert!(!generated.contains("ConstructProvider"));
        assert!(!generated.contains("\"internal\""));
    }
}
