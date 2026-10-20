// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Expansion of non-generic reflected enum declarations.

use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use syn::DeriveInput;
use syn::Ident;
use syn::Lifetime;
use syn::LitInt;
use syn::Meta;
use syn::Token;
use syn::parse::Parser;
use syn::parse_quote;
use syn::parse2;
use syn::punctuated::Punctuated;

use super::enum_repr_ir::EnumReprIr;
use crate::expand::ExpansionContext;
use crate::ir::GenericKindIr;
use crate::ir::HelperName;
use crate::ir::TypeDeclarationIr;
use crate::ir::TypeDeclarationKindIr;
use crate::ir::VariantIr;
use crate::ir::VariantKindIr;

/// Expands an enum root, its variants, and safe active-variant field adapters.
///
/// # Parameters
///
/// - `declaration`: Validated enum declaration and reflection attributes.
/// - `context`: Expansion context containing the caller-visible runtime path.
///
/// # Returns
///
/// Returns generated enum reflection, access, construction, and registration
/// items, or a diagnostic if retained generic syntax is malformed.
///
/// # Errors
///
/// Returns a syntax diagnostic when the declaration is not an enum or retained
/// generic syntax cannot be parsed.
pub(crate) fn expand(declaration: TypeDeclarationIr, context: &ExpansionContext) -> syn::Result<TokenStream> {
    expand_v4(declaration, context)
}

fn expand_v4(declaration: TypeDeclarationIr, context: &ExpansionContext) -> syn::Result<TokenStream> {
    if declaration.kind != TypeDeclarationKindIr::Enum {
        return Err(syn::Error::new(
            declaration.span,
            "cannot expand Reflect for non-enum declaration",
        ));
    }
    let facade = context.facade().clone();
    let name = declaration.name.clone();
    let fingerprint = context.fingerprint(&declaration.retained_tokens.to_string());
    let module = format_ident!("__qubit_reflect_v4_enum_{fingerprint:016x}");
    let marker = format_ident!("TypeDefinitionMarker");
    let environment = super::generic_environment::GenericEnvironment::from_generics(&declaration.generics);
    let constructible_variants: Vec<_> = declaration
        .variants
        .iter()
        .filter(|variant| {
            !variant
                .attributes
                .iter()
                .any(|attribute| attribute.name == HelperName::Skip)
                && super::generics::variant_cfg_attributes(&declaration, variant.index).is_empty()
                && !has_skipped_source_field(&declaration, variant.index)
                && variant.fields.iter().all(|field| {
                    field
                        .attributes
                        .iter()
                        .any(|attribute| attribute.name == HelperName::Skip)
                        || super::structs::supports_construction(&field.ty, &environment)
                })
                && variant.fields.iter().all(|field| {
                    super::generics::field_cfg_attributes(&declaration, Some(variant.index), field.index).is_empty()
                })
                && variant.fields.iter().all(|field| {
                    !field
                        .attributes
                        .iter()
                        .any(|attribute| attribute.name == HelperName::Skip)
                })
        })
        .map(|variant| variant.index)
        .collect();
    let thread_safe_construction = declaration.generics.params.is_empty()
        && declaration.variants.iter().all(|variant| {
            !variant
                .attributes
                .iter()
                .any(|attribute| attribute.name == HelperName::Skip)
                && super::generics::variant_cfg_attributes(&declaration, variant.index).is_empty()
                && !has_skipped_source_field(&declaration, variant.index)
                && variant.fields.iter().all(|field| {
                    !field
                        .attributes
                        .iter()
                        .any(|attribute| attribute.name == HelperName::Skip)
                        && super::structs::supports_construction(&field.ty, &environment)
                        && super::generics::field_cfg_attributes(&declaration, Some(variant.index), field.index)
                            .is_empty()
                })
        });
    let integer_repr = declaration
        .variants
        .iter()
        .all(|variant| variant.kind == VariantKindIr::Unit)
        .then(|| {
            declaration.generics.params.is_empty().then(|| {
                enum_representations(&declaration.retained_tokens)
                    .iter()
                    .find_map(EnumReprIr::integer_name)
            })
        })
        .flatten()
        .flatten();
    let mut generics = super::generics::parse_type_generics(&declaration)?;
    {
        let where_clause = generics.make_where_clause();
        for parameter in declaration
            .generics
            .params
            .iter()
            .filter(|parameter| parameter.kind == GenericKindIr::Lifetime)
        {
            let lifetime = Lifetime::new(&format!("'{}", parameter.name), parameter.span);
            where_clause.predicates.push(parse_quote!(#lifetime: 'static));
        }
        for parameter in declaration
            .generics
            .params
            .iter()
            .filter(|parameter| parameter.kind == GenericKindIr::Type)
        {
            let parameter = Ident::new(&parameter.name, parameter.span);
            where_clause.predicates.push(parse_quote!(#parameter: 'static));
        }
        for variant in &declaration.variants {
            for field in &variant.fields {
                if field
                    .attributes
                    .iter()
                    .any(|attribute| attribute.name == HelperName::Skip)
                    || !super::structs::supports_construction(&field.ty, &environment)
                    || !super::generics::field_cfg_attributes(&declaration, Some(variant.index), field.index).is_empty()
                {
                    continue;
                }
                let ty = &field.ty.tokens;
                where_clause
                    .predicates
                    .push(parse_quote!(#ty: #facade::__private::codegen_v4::descriptor::Reflect));
            }
        }
    }
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let mut methods = Vec::new();
    let mut variant_metadata = Vec::new();
    let mut construction_plans = Vec::new();
    let active_variant_identity = format_ident!("__qubit_reflect_active_variant_identity");
    let source_variants = source_enum(&declaration);
    let active_variant_arms = source_variants.iter().enumerate().map(|(index, variant)| {
        let name = &variant.ident;
        let pattern = match &variant.fields {
            syn::Fields::Unit => quote!(Self::#name),
            syn::Fields::Unnamed(_) => quote!(Self::#name(..)),
            syn::Fields::Named(_) => quote!(Self::#name { .. }),
        };
        let cfg = variant
            .attrs
            .iter()
            .filter(|attribute| attribute.path().is_ident("cfg") || attribute.path().is_ident("cfg_attr"));
        let name = name.to_string();
        quote! {
            #(#cfg)*
            #pattern => #facade::__private::codegen_v4::access::VariantIdentity {
                owner: ::std::any::TypeId::of::<Self>(),
                index: #index,
                name: #name,
            },
        }
    });
    methods.push(quote! {
        fn #active_variant_identity(value: &Self) -> #facade::__private::codegen_v4::access::VariantIdentity {
            match value { #(#active_variant_arms)* }
        }
    });
    for variant in declaration.variants.iter().filter(|variant| {
        !variant
            .attributes
            .iter()
            .any(|attribute| attribute.name == HelperName::Skip)
    }) {
        let index = variant.index;
        let variant_name = &variant.name;
        let active = format_ident!("is_variant_{index}");
        let static_variant_metadata_fn = format_ident!("static_variant_metadata_{index}");
        let cfg = super::generics::variant_cfg_attributes(&declaration, index);
        let variant_pattern = match variant.kind {
            VariantKindIr::Unit => quote!(Self::#variant_name),
            VariantKindIr::Tuple => quote!(Self::#variant_name(..)),
            VariantKindIr::Struct => quote!(Self::#variant_name { .. }),
        };
        methods.push(quote! {
            #(#cfg)*
            fn #active(value: &Self) -> bool { matches!(value, #variant_pattern) }
            #(#cfg)*
            fn #static_variant_metadata_fn() -> &'static #facade::__private::codegen_v4::access::VariantMetadata {
                <Self as #facade::__private::codegen_v4::descriptor::Reflect>::__reflect_registration()
                    .variant(#index).expect("derived variant metadata must exist")
            }
        });
        let fields: Vec<_> = variant
            .fields
            .iter()
            .filter(|field| {
                !field
                    .attributes
                    .iter()
                    .any(|attribute| attribute.name == HelperName::Skip)
            })
            .collect();
        let mut field_metadata = Vec::new();
        let mut construct_fields = Vec::new();
        for field in fields {
            let field_index = field.index;
            let get = format_ident!("get_variant_{index}_field_{field_index}");
            let get_mut = format_ident!("get_mut_variant_{index}_field_{field_index}");
            let ty = &field.ty.tokens;
            let binding = format_ident!("field_value");
            let (pattern, mutable_pattern) = if let Some(field_name) = &field.name {
                (
                    quote!(Self::#variant_name { #field_name: #binding, .. } => #binding, _ => unreachable!("field is only active for its variant")),
                    quote!(Self::#variant_name { #field_name: #binding, .. } => #binding, _ => unreachable!("field is only active for its variant")),
                )
            } else {
                (
                    tuple_field_patterns(&declaration, variant.index, field_index, variant_name, &binding),
                    tuple_field_patterns(&declaration, variant.index, field_index, variant_name, &binding),
                )
            };
            let cfg_field = super::generics::field_cfg_attributes(&declaration, Some(variant.index), field_index);
            methods.push(quote! {
                #(#cfg_field)*
                fn #get<'a>(value: &'a Self) -> &'a #ty {
                    match value { #pattern }
                }
                #(#cfg_field)*
                fn #get_mut<'a>(value: &'a mut Self) -> &'a mut #ty {
                    match value { #mutable_pattern }
                }
            });
            let source_name = field
                .name
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| field_index.to_string());
            let visibility = super::structs::v4_visibility(&field.visibility, &facade, field.span);
            let expression = super::expression_codegen::type_expression(&field.ty, &environment, &facade);
            let adapter = quote!(::std::boxed::Box::leak(::std::boxed::Box::new(
                #facade::__private::codegen_v4::access::TypedFieldAdapter::<Self, #ty>::for_variant(
                    Self::#get, Self::#get_mut, Self::#active_variant_identity,
                )
            )));
            let local_dynamic =
                if super::structs::supports_construction(&field.ty, &environment) && cfg_field.is_empty() {
                    quote!(.with_local_dynamic(#adapter))
                } else {
                    TokenStream::new()
                };
            let thread_safe_dynamic = if thread_safe_construction {
                quote!(.with_thread_safe_dynamic(#adapter))
            } else {
                TokenStream::new()
            };
            let access = quote!(#facade::__private::codegen_v4::access::FieldAccess::new(true, true, true, true));
            field_metadata.push(quote! {
                #(#cfg_field)*
                #facade::__private::codegen_v4::access::FieldMetadata::typed_variant::<Self, #ty>(
                    #field_index, #source_name, #visibility, #expression, #adapter,
                    Self::#static_variant_metadata_fn, #access,
                ) #local_dynamic #thread_safe_dynamic
            });
            let constructible = constructible_variants.contains(&index);
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
                #(#cfg_field)*
                #facade::__private::codegen_v4::construct::ConstructField::typed::<#ty>(
                    #source_name, #visibility, #field_policy, #update_policy,
                )
            });
        }
        let shape = match variant.kind {
            VariantKindIr::Unit => {
                quote!(#facade::__private::codegen_v4::access::VariantShape::Unit)
            }
            VariantKindIr::Tuple => {
                quote!(#facade::__private::codegen_v4::access::VariantShape::Tuple)
            }
            VariantKindIr::Struct => {
                quote!(#facade::__private::codegen_v4::access::VariantShape::Struct)
            }
        };
        let variant_visibility = super::structs::v4_visibility(&declaration.visibility, &facade, declaration.span);
        let variant_name_text = variant_name.to_string();
        let origin = if variant.discriminant.is_some() {
            quote!(#facade::__private::codegen_v4::access::DiscriminantOrigin::Explicit)
        } else {
            quote!(#facade::__private::codegen_v4::access::DiscriminantOrigin::Implicit)
        };
        let numeric = numeric_discriminant_v4(&name, variant_name, integer_repr, &facade);
        let variant_discriminant =
            quote!(#facade::__private::codegen_v4::access::VariantDiscriminant::new(#origin, #numeric));
        variant_metadata.push(quote! {
            #(#cfg)*
            #facade::__private::codegen_v4::access::VariantMetadata::new(
                #index, #variant_name_text, #shape, #variant_visibility, #variant_discriminant,
                ::std::boxed::Box::leak(::std::vec![#(#field_metadata),*].into_boxed_slice()),
                ::std::boxed::Box::leak(::std::boxed::Box::new(
                    #facade::__private::codegen_v4::access::TypedVariantAdapter::<Self>::new(Self::#active)
                )),
            )
        });
        let construction_shape = match variant.kind {
            VariantKindIr::Unit => {
                quote!(#facade::__private::codegen_v4::construct::ConstructionShape::Unit)
            }
            VariantKindIr::Tuple => {
                quote!(#facade::__private::codegen_v4::construct::ConstructionShape::Positional)
            }
            VariantKindIr::Struct => {
                quote!(#facade::__private::codegen_v4::construct::ConstructionShape::Named)
            }
        };
        let (provider_methods, providers, availability) = enum_variant_provider(
            variant,
            &facade,
            constructible_variants.contains(&index),
            thread_safe_construction,
        );
        methods.push(provider_methods);
        construction_plans.push(quote! {
            #(#cfg)*
            #facade::__private::codegen_v4::construct::ConstructionPlan::for_variant::<Self>(
                #index, #construction_shape,
                ::std::boxed::Box::leak(::std::vec![#(#construct_fields),*].into_boxed_slice()),
                #providers,
                #availability,
            )
        });
    }
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
                    #facade::__private::codegen_v4::expression::TypeDefinitionKind::Enum,
                    generic_definition,
                ),
            ));
            let generic_arguments: &'static [#facade::__private::codegen_v4::expression::GenericArgumentMetadata] = #generic_arguments;
            (Some(#type_definition_ident), generic_arguments)
        })
    };
    let type_submission = if declaration.generics.params.is_empty() {
        quote!(#facade::__private::codegen_v4::registration::submit_type!(#name);)
    } else {
        let registration_fn = format_ident!("{}_registration", module);
        let rust_name = name.to_string();
        quote! {
            #[doc(hidden)]
            fn #registration_fn() -> #facade::__private::codegen_v4::registration::Registration {
                static VALUE: ::std::sync::OnceLock<
                    ::std::boxed::Box<#facade::__private::codegen_v4::registration::TypeDefinitionRegistration<#module::#marker>>,
                > = ::std::sync::OnceLock::new();
                let registration = VALUE.get_or_init(|| {
                    let generics = ::std::boxed::Box::leak(::std::boxed::Box::new(#generic_definition));
                    ::std::boxed::Box::new(
                        #facade::__private::codegen_v4::registration::TypeDefinitionRegistration::new(
                            #rust_name,
                            concat!(module_path!(), "::", stringify!(#name)),
                            #facade::__private::codegen_v4::expression::TypeDefinitionKind::Enum,
                            generics,
                        ),
                    )
                });
                #facade::__private::codegen_v4::registration::Registration::type_definition(registration)
            }
            #facade::__private::codegen_v4::registration::submit_type_definition!(#rust_name, #registration_fn);
        }
    };
    Ok(quote! {
        #[allow(non_camel_case_types)]
        mod #module { pub struct #marker; }
        impl #impl_generics #name #type_generics #where_clause {
            #(#methods)*
        }
        impl #impl_generics #facade::__private::codegen_v4::descriptor::Reflect for #name #type_generics #where_clause {
            fn __reflect_registration() -> &'static #facade::__private::codegen_v4::descriptor::TypeRegistration<Self> {
                #facade::__private::codegen_v4::descriptor::TypeRegistration::<Self>::get_or_init_with(|| {
                    let (type_definition, generic_arguments) = #type_definition_and_arguments;
                    #facade::__private::codegen_v4::descriptor::TypeRegistration::<Self>::with_shape(
                        ::std::boxed::Box::leak(::std::boxed::Box::new(
                            #facade::__private::codegen_v4::access::TypeKindMetadata::Enum(
                            ::std::boxed::Box::leak(::std::vec![#(#variant_metadata),*].into_boxed_slice())
                            )
                        )),
                        type_definition,
                        ::std::boxed::Box::leak(::std::vec![#(#construction_plans),*].into_boxed_slice()),
                    ).with_generic_arguments(generic_arguments)
                })
            }
        }
        #type_submission
    })
}

#[allow(dead_code)]
fn numeric_discriminant_v4(
    enum_name: &Ident,
    variant_name: &Ident,
    repr: Option<&str>,
    facade: &TokenStream,
) -> TokenStream {
    let Some(repr) = repr else {
        return quote!(None);
    };
    let variant = match repr {
        "i8" => quote!(I8),
        "i16" => quote!(I16),
        "i32" => quote!(I32),
        "i64" => quote!(I64),
        "i128" => quote!(I128),
        "isize" => quote!(Isize),
        "u8" => quote!(U8),
        "u16" => quote!(U16),
        "u32" => quote!(U32),
        "u64" => quote!(U64),
        "u128" => quote!(U128),
        "usize" => quote!(Usize),
        _ => return quote!(None),
    };
    let repr = Ident::new(repr, variant_name.span());
    quote!(Some(#facade::__private::codegen_v4::access::NumericDiscriminant::#variant(
        #enum_name::#variant_name as #repr
    )))
}

fn source_variant_fields(declaration: &TypeDeclarationIr, variant_index: usize) -> syn::Fields {
    let input = source_enum(declaration);
    input
        .get(variant_index)
        .expect("variant indices follow source order")
        .fields
        .clone()
}

fn source_enum(declaration: &TypeDeclarationIr) -> Vec<syn::Variant> {
    let input: DeriveInput =
        parse2(declaration.retained_tokens.clone()).expect("the parser retains a valid enum declaration");
    let syn::Data::Enum(data) = input.data else {
        unreachable!("enum expansion receives an enum declaration");
    };
    data.variants.into_iter().collect()
}

/// Builds tuple-variant patterns that account for source fields removed by
/// conditional compilation. Each cfg alternative has its own pattern because
/// Rust does not permit cfg attributes on individual tuple-pattern elements.
fn tuple_field_patterns(
    declaration: &TypeDeclarationIr,
    variant_index: usize,
    field_index: usize,
    variant_name: &Ident,
    binding: &Ident,
) -> TokenStream {
    let mut conditional_prefix = Vec::new();
    let mut fixed_prefix = Vec::new();
    for source_index in 0..field_index {
        let cfg = super::generics::field_cfg_attributes(declaration, Some(variant_index), source_index);
        if cfg.is_empty() {
            fixed_prefix.push(source_index);
        } else {
            conditional_prefix.push((source_index, field_presence_predicate(&cfg)));
        }
    }

    let alternatives = 1_usize << conditional_prefix.len();
    let arms = (0..alternatives).map(|mask| {
        let mut cfg_terms = Vec::new();
        let mut prefix = Vec::new();
        let mut conditional_index = 0;
        for source_index in 0..field_index {
            if fixed_prefix.contains(&source_index) {
                prefix.push(quote!(_));
                continue;
            }
            let (_, predicate) = &conditional_prefix[conditional_index];
            let present = mask & (1 << conditional_index) != 0;
            if present {
                cfg_terms.push(quote!(#predicate));
                prefix.push(quote!(_));
            } else {
                cfg_terms.push(quote!(not(#predicate)));
            }
            conditional_index += 1;
        }
        let cfg = if cfg_terms.is_empty() {
            TokenStream::new()
        } else {
            quote!(#[cfg(all(#(#cfg_terms),*))])
        };
        let pattern = if prefix.is_empty() {
            quote!(Self::#variant_name(#binding, ..))
        } else {
            quote!(Self::#variant_name(#(#prefix,)* #binding, ..))
        };
        quote!(#cfg #pattern => #binding,)
    });
    quote!(#(#arms)* _ => unreachable!("field is only active for its variant"))
}

/// Converts field cfg/cfg_attr attributes to a cfg predicate describing when
/// the field exists in the compiled enum shape.
fn field_presence_predicate(attributes: &[syn::Attribute]) -> TokenStream {
    let constraints = attributes
        .iter()
        .filter_map(attribute_presence_predicate)
        .collect::<Vec<_>>();
    quote!(all(#(#constraints),*))
}

fn attribute_presence_predicate(attribute: &syn::Attribute) -> Option<TokenStream> {
    if attribute.path().is_ident("cfg") {
        let predicate = attribute.parse_args::<Meta>().ok()?;
        return Some(quote!(#predicate));
    }
    if !attribute.path().is_ident("cfg_attr") {
        return None;
    }
    let arguments = Punctuated::<Meta, Token![,]>::parse_terminated
        .parse2(attribute.meta.require_list().ok()?.tokens.clone())
        .ok()?;
    let mut arguments = arguments.into_iter();
    let gate = arguments.next()?;
    let nested = arguments
        .filter_map(|meta| meta_presence_predicate(&meta))
        .collect::<Vec<_>>();
    if nested.is_empty() {
        return None;
    }
    let nested = quote!(all(#(#nested),*));
    Some(quote!(any(not(#gate), #nested)))
}

fn meta_presence_predicate(meta: &Meta) -> Option<TokenStream> {
    if meta.path().is_ident("cfg") {
        let Meta::List(list) = meta else {
            return None;
        };
        let predicate = syn::parse2::<Meta>(list.tokens.clone()).ok()?;
        return Some(quote!(#predicate));
    }
    if !meta.path().is_ident("cfg_attr") {
        return None;
    }
    let Meta::List(list) = meta else {
        return None;
    };
    let arguments = Punctuated::<Meta, Token![,]>::parse_terminated
        .parse2(list.tokens.clone())
        .ok()?;
    let mut arguments = arguments.into_iter();
    let gate = arguments.next()?;
    let nested = arguments
        .filter_map(|meta| meta_presence_predicate(&meta))
        .collect::<Vec<_>>();
    if nested.is_empty() {
        return None;
    }
    let nested = quote!(all(#(#nested),*));
    Some(quote!(any(not(#gate), #nested)))
}

fn has_skipped_source_field(declaration: &TypeDeclarationIr, variant_index: usize) -> bool {
    source_variant_fields(declaration, variant_index).iter().any(|field| {
        field.attrs.iter().any(|attribute| {
            attribute.path().is_ident("reflect")
                && attribute
                    .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
                    .is_ok_and(|items| items.iter().any(|item| item.path().is_ident("skip")))
        })
    })
}

fn enum_variant_provider(
    variant: &VariantIr,
    facade: &TokenStream,
    constructible: bool,
    thread_safe: bool,
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
    let (local_methods, local_provider) = enum_variant_provider_mode(variant, facade, "local", quote!(Local));
    if thread_safe {
        let (thread_safe_methods, thread_safe_provider) =
            enum_variant_provider_mode(variant, facade, "thread_safe", quote!(ThreadSafe));
        let providers = quote!({
            let providers = #facade::__private::codegen_v4::construct::ConstructionProviders::new::<Self>(Some(#local_provider));
            providers.with_thread_safe::<Self>(#thread_safe_provider)
        });
        return (
            quote!(#local_methods #thread_safe_methods),
            providers,
            quote!(#facade::__private::codegen_v4::construct::ConstructionAvailability::Available),
        );
    }
    (
        local_methods,
        quote!(#facade::__private::codegen_v4::construct::ConstructionProviders::new::<Self>(Some(#local_provider))),
        quote!(#facade::__private::codegen_v4::construct::ConstructionAvailability::Available),
    )
}

fn enum_variant_provider_mode(
    variant: &VariantIr,
    facade: &TokenStream,
    suffix: &str,
    mode: TokenStream,
) -> (TokenStream, TokenStream) {
    let variant_name = &variant.name;
    let index = variant.index;
    let construct_name = format_ident!("construct_variant_{index}_{suffix}");
    let update_name = format_ident!("update_variant_{index}_{suffix}");
    let fields: Vec<_> = variant.fields.iter().collect();
    let variables: Vec<_> = (0..fields.len())
        .map(|position| format_ident!("field_{position}"))
        .collect();
    let mut downcasts = Vec::new();
    let mut updates = Vec::new();
    for (position, (field, variable)) in fields.iter().zip(&variables).enumerate() {
        let ty = &field.ty.tokens;
        let position = syn::Index::from(position);
        downcasts.push(quote!(let #variable = fields.get_mut(#position).and_then(Option::take)
            .expect("validated construction field must be present")
            .downcast::<#ty>().unwrap_or_else(|_| unreachable!("validated construction field type"));));
        updates.push(
            quote!(if let Some(replacement) = fields.get_mut(#position).and_then(Option::take) {
                #variable = replacement.downcast::<#ty>()
                    .unwrap_or_else(|_| unreachable!("validated update field type"));
            }),
        );
    }
    let (destructure, updated_value, created_value) = match variant.kind {
        VariantKindIr::Unit => (
            quote!(let Self::#variant_name = value else { unreachable!("validated active enum variant") };),
            quote!(Self::#variant_name),
            quote!(Self::#variant_name),
        ),
        VariantKindIr::Tuple => (
            quote!(let Self::#variant_name(#(mut #variables),*) = value else { unreachable!("validated active enum variant") };),
            quote!(Self::#variant_name(#(#variables),*)),
            quote!(Self::#variant_name(#(#variables),*)),
        ),
        VariantKindIr::Struct => {
            let field_names: Vec<_> = fields
                .iter()
                .map(|field| field.name.as_ref().expect("named variant field"))
                .collect();
            (
                quote!(let Self::#variant_name { #(#field_names: mut #variables),* } = value else { unreachable!("validated active enum variant") };),
                quote!(Self::#variant_name { #(#field_names: #variables),* }),
                quote!(Self::#variant_name { #(#field_names: #variables),* }),
            )
        }
    };
    let methods = quote! {
        fn #construct_name(mut fields: ::std::vec::Vec<Option<#facade::__private::codegen_v4::value::DynamicOwned<#facade::__private::codegen_v4::value::#mode>>>) -> Self {
            #(#downcasts)*
            #created_value
        }
        fn #update_name(value: Self, mut fields: ::std::vec::Vec<Option<#facade::__private::codegen_v4::value::DynamicOwned<#facade::__private::codegen_v4::value::#mode>>>) -> Self {
            #destructure
            #(#updates)*
            #updated_value
        }
    };
    let provider = quote! {
        ::std::boxed::Box::leak(::std::boxed::Box::new(
            #facade::__private::codegen_v4::construct::ConstructProvider::<Self, #facade::__private::codegen_v4::value::#mode>::new(
                Self::#construct_name, Self::#update_name,
            )
        ))
    };
    (methods, provider)
}

/// Returns the unique supported representation hints in canonical order.
#[must_use]
fn enum_representations(tokens: &TokenStream) -> Vec<EnumReprIr> {
    let Ok(input) = parse2::<DeriveInput>(tokens.clone()) else {
        return Vec::new();
    };
    let mut representations = Vec::new();
    for attribute in input.attrs.iter().filter(|attribute| attribute.path().is_ident("repr")) {
        let Meta::List(list) = &attribute.meta else {
            continue;
        };
        let Ok(values) = list.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated) else {
            continue;
        };
        representations.extend(values.iter().filter_map(parse_enum_representation));
    }
    representations.sort_unstable();
    representations.dedup();
    representations
}

/// Parses one compiler-validated `repr` component into structural metadata.
///
/// # Parameters
///
/// - `meta`: One representation path or alignment list.
///
/// # Returns
///
/// Returns the recognized representation, or `None` for unsupported metadata.
#[must_use]
fn parse_enum_representation(meta: &Meta) -> Option<EnumReprIr> {
    match meta {
        Meta::Path(path) if path.is_ident("Rust") => Some(EnumReprIr::Rust),
        Meta::Path(path) if path.is_ident("C") => Some(EnumReprIr::C),
        Meta::Path(path) if path.is_ident("transparent") => Some(EnumReprIr::Transparent),
        Meta::Path(path) if path.is_ident("i8") => Some(EnumReprIr::I8),
        Meta::Path(path) if path.is_ident("i16") => Some(EnumReprIr::I16),
        Meta::Path(path) if path.is_ident("i32") => Some(EnumReprIr::I32),
        Meta::Path(path) if path.is_ident("i64") => Some(EnumReprIr::I64),
        Meta::Path(path) if path.is_ident("i128") => Some(EnumReprIr::I128),
        Meta::Path(path) if path.is_ident("isize") => Some(EnumReprIr::Isize),
        Meta::Path(path) if path.is_ident("u8") => Some(EnumReprIr::U8),
        Meta::Path(path) if path.is_ident("u16") => Some(EnumReprIr::U16),
        Meta::Path(path) if path.is_ident("u32") => Some(EnumReprIr::U32),
        Meta::Path(path) if path.is_ident("u64") => Some(EnumReprIr::U64),
        Meta::Path(path) if path.is_ident("u128") => Some(EnumReprIr::U128),
        Meta::Path(path) if path.is_ident("usize") => Some(EnumReprIr::Usize),
        Meta::List(list) if list.path.is_ident("align") => {
            let alignment = parse2::<LitInt>(list.tokens.clone()).ok()?;
            alignment.base10_parse().ok().map(EnumReprIr::Align)
        }
        _ => None,
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
    fn test_enum_emission_uses_codegen_v4_only() {
        let parsed = parse_and_validate_declaration(
            MacroKind::Derive,
            TokenStream::new(),
            quote! {
                enum Fixture<T> {
                    Unit,
                    Value(T),
                    #[reflect(skip)]
                    Hidden { value: u8 },
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

        assert!(generated.contains("codegen_v4"), "enum tokens must use the v4 protocol");
        assert!(
            !generated.contains(concat!("codegen_", "v3")),
            "enum tokens must not use the v3 protocol"
        );
        assert!(generated.contains("TypedFieldAdapter"));
        assert!(generated.contains("variant_metadata"));
        assert!(generated.contains("with_local_dynamic"));
        assert!(!generated.contains("with_thread_safe_dynamic"));
        for forbidden in ["TypeIdentity", "FragmentIdentity", "RuntimeIdentity", "inventory"] {
            assert!(
                !generated.contains(forbidden),
                "generated tokens must not mention {forbidden}"
            );
        }
    }

    #[test]
    fn test_skipped_source_variant_fields_preserve_patterns_and_disable_providers() {
        let parsed = parse_and_validate_declaration(
            MacroKind::Derive,
            TokenStream::new(),
            quote! {
                enum Fixture {
                    Named {
                        value: u8,
                        #[reflect(skip)]
                        internal: String,
                    },
                    Tuple(u8, #[reflect(skip)] String, u16),
                }
            },
        )
        .expect("the enum fixture should parse");
        let DeclarationIr::Type(declaration) = parsed.declaration else {
            panic!("the fixture should be an enum");
        };
        let generated = expand(declaration, &ExpansionContext::for_tests())
            .expect("the enum should expand")
            .to_string();

        assert!(generated.contains("ConstructionAvailability :: Unavailable"));
        assert!(!generated.contains("ConstructProvider"));
        assert!(generated.contains("Self :: Tuple (field_value , ..)"));
        assert!(generated.contains("Self :: Tuple (_ , _ , field_value , ..)"));
        assert!(!generated.contains("\"internal\""));
    }
}
