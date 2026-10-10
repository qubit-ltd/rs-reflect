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
use syn::parse2;
use syn::parse_quote;
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
    if declaration.kind != TypeDeclarationKindIr::Enum {
        return Err(syn::Error::new(
            declaration.span,
            "cannot expand Reflect for non-enum declaration",
        ));
    }
    let facade = context.facade().clone();
    let name = declaration.name.clone();
    let reflected_field_types = super::generics::reflected_field_types(&declaration);
    let transparently_reflected_parameters = super::generics::transparently_reflected_type_parameters(&declaration);
    let type_parameters: Vec<_> = declaration
        .generics
        .params
        .iter()
        .filter(|parameter| parameter.kind == GenericKindIr::Type)
        .map(|parameter| Ident::new(&parameter.name, parameter.span))
        .collect();
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
        for parameter in &type_parameters {
            where_clause.predicates.push(parse_quote!(#parameter: 'static));
        }
        for field_type in &reflected_field_types {
            let field_type = &field_type.tokens;
            where_clause
                .predicates
                .push(parse_quote!(#field_type: #facade::__private::codegen_v3::Reflect));
        }
        for parameter in &transparently_reflected_parameters {
            where_clause
                .predicates
                .push(parse_quote!(#parameter: #facade::__private::codegen_v3::Reflect));
        }
    }
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let self_type = quote!(#name #type_generics);
    let fingerprint = context.fingerprint(&declaration.retained_tokens.to_string());
    let registration_module = format_ident!("__qubit_reflect_enum_registration_{fingerprint:016x}");
    let query_name = declaration
        .attributes
        .iter()
        .find_map(|attribute| attribute.rename())
        .unwrap_or(&name.to_string())
        .to_owned();
    let capability_function = format_ident!("__qubit_reflect_capabilities_{fingerprint:016x}");
    let capability_resolver = quote!(<#self_type>::#capability_function);
    let capability_definition = super::structs::capabilities(&declaration, &facade, &capability_function);
    let representations = enum_representations(&declaration.retained_tokens);
    let integer_repr = declaration
        .variants
        .iter()
        .all(|variant| variant.kind == VariantKindIr::Unit)
        .then(|| {
            declaration
                .generics
                .params
                .is_empty()
                .then(|| representations.iter().find_map(EnumReprIr::integer_name))
        })
        .flatten()
        .flatten();
    if declaration
        .attributes
        .iter()
        .any(|attribute| attribute.name == HelperName::Opaque)
    {
        let generic_definition_provider = super::generics::definition_provider(&declaration, &facade);
        let type_definition_provider = super::generics::type_definition_provider(&declaration, &facade, fingerprint);
        let registration = registration(
            &facade,
            &name,
            &registration_module,
            fingerprint,
            !declaration.generics.params.is_empty(),
        );
        let descriptor = if declaration.generics.params.is_empty() {
            quote!(#facade::__private::codegen_v3::descriptor::with_capabilities(
                #facade::__private::codegen_v3::descriptor::opaque_root::<Self>(#query_name),
                #capability_resolver,
            ))
        } else {
            let generic = super::generics::concrete_descriptor(&declaration, &facade);
            let definition = super::generics::type_definition_provider_name(&declaration);
            quote!(#facade::__private::codegen_v3::descriptor::with_type_definition(
                #facade::__private::codegen_v3::descriptor::with_concrete_generic(
                    #facade::__private::codegen_v3::descriptor::with_capabilities(
                        #facade::__private::codegen_v3::descriptor::opaque_root::<Self>(#query_name),
                        #capability_resolver,
                    ),
                    ::std::boxed::Box::leak(::std::boxed::Box::new(#generic)),
                ),
                #definition,
            ))
        };
        return Ok(quote! {
            impl #impl_generics #name #type_generics #where_clause {
                #capability_definition
            }
            impl #impl_generics #facade::__private::codegen_v3::Reflect for #name #type_generics #where_clause {
                fn type_descriptor() -> &'static #facade::__private::codegen_v3::TypeDescriptor {
                    #facade::__private::codegen_v3::descriptor::intern_type::<Self>(|| {
                        #descriptor
                    })
                }
            }
            #registration
            #generic_definition_provider
            #type_definition_provider
        });
    }
    let thread_safe = declaration
        .attributes
        .iter()
        .any(|attribute| attribute.name == HelperName::ThreadSafe);
    let adapters = declaration
        .variants
        .iter()
        .filter(|variant| {
            !variant
                .attributes
                .iter()
                .any(|attribute| attribute.name == HelperName::Skip)
        })
        .flat_map(|variant| adapters(&name, variant, &facade, thread_safe));
    let construction_adapters = declaration
        .variants
        .iter()
        .map(|variant| super::construction::variant_adapters(variant, &facade, thread_safe));
    let variants = declaration
        .variants
        .iter()
        .filter(|variant| {
            !variant
                .attributes
                .iter()
                .any(|attribute| attribute.name == HelperName::Skip)
        })
        .map(|variant| {
            variant_descriptor(
                &name,
                &quote!(#name #type_generics),
                variant,
                &facade,
                integer_repr,
                thread_safe,
            )
        });
    let representation_values = representations
        .iter()
        .map(|representation| representation.descriptor_tokens(&facade));
    let enum_descriptor = if declaration.generics.params.is_empty() {
        quote!({
            let representations = ::std::boxed::Box::leak(
                ::std::vec![#(#representation_values),*].into_boxed_slice(),
            );
            #facade::__private::codegen_v3::descriptor::with_capabilities(
                #facade::__private::codegen_v3::descriptor::enum_type_with_repr::<Self>(
                    #query_name,
                    variants,
                    representations,
                ),
                #capability_resolver,
            )
        })
    } else {
        let generic = super::generics::concrete_descriptor(&declaration, &facade);
        let definition = super::generics::type_definition_provider_name(&declaration);
        quote! {
            {
                let representations = ::std::boxed::Box::leak(
                    ::std::vec![#(#representation_values),*].into_boxed_slice(),
                );
                #facade::__private::codegen_v3::descriptor::with_type_definition(
                    #facade::__private::codegen_v3::descriptor::with_concrete_generic(
                        #facade::__private::codegen_v3::descriptor::with_capabilities(
                            #facade::__private::codegen_v3::descriptor::enum_type_with_repr::<Self>(
                                #query_name,
                                variants,
                                representations,
                            ),
                            #capability_resolver,
                        ),
                        ::std::boxed::Box::leak(::std::boxed::Box::new(#generic)),
                    ),
                    #definition,
                )
            }
        }
    };
    let registration = registration(
        &facade,
        &name,
        &registration_module,
        fingerprint,
        !declaration.generics.params.is_empty(),
    );
    let generic_definition_provider = super::generics::definition_provider(&declaration, &facade);
    let type_definition_provider = super::generics::type_definition_provider(&declaration, &facade, fingerprint);
    let root_descriptor = quote! {
        impl #impl_generics #name #type_generics #where_clause {
            #capability_definition
            #(#adapters)*
            #(#construction_adapters)*
        }

        impl #impl_generics #facade::__private::codegen_v3::Reflect for #name #type_generics #where_clause {
            fn type_descriptor() -> &'static #facade::__private::codegen_v3::TypeDescriptor {
                #facade::__private::codegen_v3::descriptor::intern_type::<Self>(|| {
                    let variants = ::std::boxed::Box::leak(::std::vec![#(#variants),*].into_boxed_slice());
                    #enum_descriptor
                })
            }
        }

        #registration
        #generic_definition_provider
        #type_definition_provider
    };
    Ok(root_descriptor)
}

/// Emits the static registry fragment for one concrete derived enum root.
///
/// # Parameters
///
/// - `facade`: Runtime facade path used by the generated registration code.
/// - `name`: Enum identifier used to resolve its runtime type.
/// - `module`: Unique generated module identifier.
/// - `fingerprint`: Stable normalized-input fingerprint.
/// - `has_generics`: Whether generic parameters prevent static registration.
///
/// # Returns
///
/// Returns the registration module, or an empty stream for generic enums.
#[must_use]
fn registration(
    facade: &TokenStream,
    name: &Ident,
    module: &Ident,
    fingerprint: u64,
    has_generics: bool,
) -> TokenStream {
    if has_generics {
        return TokenStream::new();
    }
    quote! {
        #[doc(hidden)]
        mod #module {
            use super::*;

            fn runtime_identity() -> #facade::__private::codegen_v3::registration::RuntimeIdentity {
                #facade::__private::codegen_v3::registration::RuntimeIdentity::Type(::std::any::TypeId::of::<#name>())
            }

            fn payload() -> #facade::__private::codegen_v3::registration::FragmentPayload {
                #facade::__private::codegen_v3::registration::FragmentPayload::Type(
                    <#name as #facade::__private::codegen_v3::Reflect>::type_descriptor(),
                )
            }

            #facade::__private::codegen_v3::inventory::submit! {
                #facade::__private::codegen_v3::registration::RegistrationFragment::new(
                    #facade::__private::codegen_v3::registration::FragmentKind::Type,
                    #facade::__private::codegen_v3::registration::StaticFragmentIdentity::new(
                        env!("CARGO_PKG_NAME"), module_path!(), line!(), column!(), "type", #fingerprint,
                    ),
                    runtime_identity,
                    payload,
                )
            }
        }
    }
}

/// Generates active-variant and field-access adapters for one enum variant.
///
/// # Parameters
///
/// - `_name`: Enum identifier retained for the call site's declaration context.
/// - `variant`: Validated variant and field metadata.
/// - `facade`: Runtime facade path used in generated references.
/// - `thread_safe`: Whether thread-safe adapter variants are also emitted.
///
/// # Returns
///
/// Returns generated active-variant and field access functions.
#[must_use]
fn adapters(_name: &Ident, variant: &VariantIr, facade: &TokenStream, thread_safe: bool) -> Vec<TokenStream> {
    let variant_name = &variant.name;
    let variant_index = variant.index;
    let variant_name_text = variant_name.to_string();
    let active = format_ident!("__qubit_reflect_is_variant_{variant_index}");
    let active_pattern = match variant.kind {
        VariantKindIr::Unit => quote!(Self::#variant_name),
        VariantKindIr::Tuple => quote!(Self::#variant_name(..)),
        VariantKindIr::Struct => quote!(Self::#variant_name { .. }),
    };
    let mut definitions = vec![quote! {
        fn #active(value: #facade::__private::codegen_v3::value::ReflectedRef<'_>)
            -> ::core::result::Result<bool, #facade::__private::codegen_v3::error::TypeMismatch>
        {
            let value = value.downcast::<Self>().unwrap_or_else(|_| unreachable!("validated enum target"));
            Ok(matches!(value, #active_pattern))
        }
    }];
    definitions.extend(variant
        .fields
        .iter()
        .flat_map(|field| {
            let index = field.index;
            let get = format_ident!("__qubit_reflect_get_variant_{variant_index}_field_{index}");
            let get_mut = format_ident!("__qubit_reflect_get_mut_variant_{variant_index}_field_{index}");
            let set = format_ident!("__qubit_reflect_set_variant_{variant_index}_field_{index}");
            let set_preflight = format_ident!("__qubit_reflect_preflight_set_variant_{variant_index}_field_{index}");
            let get_thread_safe = format_ident!("__qubit_reflect_get_variant_{variant_index}_field_{index}_thread_safe");
            let get_mut_thread_safe = format_ident!("__qubit_reflect_get_mut_variant_{variant_index}_field_{index}_thread_safe");
            let set_thread_safe = format_ident!("__qubit_reflect_set_variant_{variant_index}_field_{index}_thread_safe");
            let set_preflight_thread_safe = format_ident!("__qubit_reflect_preflight_set_variant_{variant_index}_field_{index}_thread_safe");
            let ty = &field.ty.tokens;
            let binding = format_ident!("__qubit_reflect_value_{index}");
            let rust_name = field.name.as_ref().map(|value| value.to_string());
            let rust_name = match rust_name {
                Some(value) => quote!(Some(#value)),
                None => quote!(None),
            };
            let pattern = match variant.kind {
                VariantKindIr::Struct => {
                    let field_name = field.name.as_ref().expect("validated struct variant field");
                    quote!(Self::#variant_name { #field_name: #binding, .. })
                }
                VariantKindIr::Tuple => {
                    let bindings = (0..variant.fields.len())
                        .map(|position| if position == index { quote!(#binding) } else { quote!(_) });
                    quote!(Self::#variant_name(#(#bindings),*))
                }
                VariantKindIr::Unit => return Vec::new(),
            };
            let active_pattern = match variant.kind {
                VariantKindIr::Struct => quote!(Self::#variant_name { .. }),
                VariantKindIr::Tuple => quote!(Self::#variant_name(..)),
                VariantKindIr::Unit => return Vec::new(),
            };
            let inactive = quote!(#facade::__private::codegen_v3::access::FieldAccessError::inactive_variant(
                #facade::__private::codegen_v3::access::FieldIdentity::new_variant(
                    ::std::any::TypeId::of::<Self>(), ::std::any::type_name::<Self>(), #index, #rust_name,
                    #variant_index, #variant_name_text,
                ),
            ));
            let thread_safe_definitions = thread_safe.then(|| quote! {
                fn #get_thread_safe<'__qubit_reflect>(target: #facade::__private::codegen_v3::value::DynamicRef<'__qubit_reflect, #facade::__private::codegen_v3::value::ThreadSafe>)
                    -> ::core::result::Result<#facade::__private::codegen_v3::value::DynamicRef<'__qubit_reflect, #facade::__private::codegen_v3::value::ThreadSafe>, #facade::__private::codegen_v3::access::FieldAccessError>
                {
                    let value = target.downcast::<Self>().unwrap_or_else(|_| unreachable!("validated enum target"));
                    match value {
                        #pattern => Ok(#facade::__private::codegen_v3::value::DynamicRef::<#facade::__private::codegen_v3::value::ThreadSafe>::new(#binding)),
                        _ => Err(#inactive),
                    }
                }
                fn #get_mut_thread_safe<'__qubit_reflect>(target: #facade::__private::codegen_v3::value::DynamicMut<'__qubit_reflect, #facade::__private::codegen_v3::value::ThreadSafe>)
                    -> ::core::result::Result<#facade::__private::codegen_v3::value::DynamicMut<'__qubit_reflect, #facade::__private::codegen_v3::value::ThreadSafe>, #facade::__private::codegen_v3::access::FieldAccessError>
                {
                    let value = target.downcast::<Self>().unwrap_or_else(|_| unreachable!("validated enum target"));
                    match value {
                        #pattern => Ok(#facade::__private::codegen_v3::value::DynamicMut::<#facade::__private::codegen_v3::value::ThreadSafe>::new(#binding)),
                        _ => Err(#inactive),
                    }
                }
                fn #set_thread_safe(
                    target: #facade::__private::codegen_v3::value::DynamicMut<'_, #facade::__private::codegen_v3::value::ThreadSafe>,
                    replacement: #facade::__private::codegen_v3::value::DynamicOwned<#facade::__private::codegen_v3::value::ThreadSafe>,
                ) -> ::core::result::Result<(), #facade::__private::codegen_v3::access::FieldAccessError> {
                    let value = target.downcast::<Self>().unwrap_or_else(|_| unreachable!("validated enum target"));
                    let replacement = replacement.downcast::<#ty>()
                        .unwrap_or_else(|_| unreachable!("validated enum field value"));
                    match value { #pattern => { *#binding = replacement; Ok(()) }, _ => Err(#inactive) }
                }
                fn #set_preflight_thread_safe(
                    target: &#facade::__private::codegen_v3::value::DynamicMut<'_, #facade::__private::codegen_v3::value::ThreadSafe>,
                ) -> ::core::result::Result<(), #facade::__private::codegen_v3::access::FieldAccessError> {
                    let value = target.downcast_ref::<Self>()
                        .unwrap_or_else(|| unreachable!("validated enum target"));
                    match value { #active_pattern => Ok(()), _ => Err(#inactive) }
                }
            });
            vec![quote! {
                fn #get<'__qubit_reflect>(target: #facade::__private::codegen_v3::value::ReflectedRef<'__qubit_reflect>)
                    -> ::core::result::Result<#facade::__private::codegen_v3::value::ReflectedRef<'__qubit_reflect>, #facade::__private::codegen_v3::access::FieldAccessError>
                {
                    let value = target.downcast::<Self>().unwrap_or_else(|_| unreachable!("validated enum target"));
                    match value { #pattern => Ok(#facade::__private::codegen_v3::value::ReflectedRef::new(#binding)), _ => Err(#inactive) }
                }
                fn #get_mut<'__qubit_reflect>(target: #facade::__private::codegen_v3::value::ReflectedMut<'__qubit_reflect>)
                    -> ::core::result::Result<#facade::__private::codegen_v3::value::ReflectedMut<'__qubit_reflect>, #facade::__private::codegen_v3::access::FieldAccessError>
                {
                    let value = target.downcast::<Self>().unwrap_or_else(|_| unreachable!("validated enum target"));
                    match value { #pattern => Ok(#facade::__private::codegen_v3::value::ReflectedMut::new(#binding)), _ => Err(#inactive) }
                }
                fn #set(target: #facade::__private::codegen_v3::value::ReflectedMut<'_>, replacement: #facade::__private::codegen_v3::value::ReflectedOwned)
                    -> ::core::result::Result<(), #facade::__private::codegen_v3::access::FieldAccessError>
                {
                    let value = target.downcast::<Self>().unwrap_or_else(|_| unreachable!("validated enum target"));
                    let replacement = #facade::__private::codegen_v3::value::ReflectedOwned::downcast::<#ty>(replacement)
                        .unwrap_or_else(|_| unreachable!("validated enum field value"));
                    match value { #pattern => { *#binding = replacement; Ok(()) }, _ => Err(#inactive) }
                }
                fn #set_preflight(target: &#facade::__private::codegen_v3::value::ReflectedMut<'_>)
                    -> ::core::result::Result<(), #facade::__private::codegen_v3::access::FieldAccessError>
                {
                    let value = target.downcast_ref::<Self>()
                        .unwrap_or_else(|| unreachable!("validated enum target"));
                    match value { #active_pattern => Ok(()), _ => Err(#inactive) }
                }
                #thread_safe_definitions
            }]
        })
        .collect::<Vec<_>>());
    definitions
}

/// Generates the descriptor for one reflected enum variant.
///
/// # Parameters
///
/// - `name`: Enum identifier used for discriminant expressions.
/// - `self_type`: Fully parameterized enum type tokens.
/// - `variant`: Validated variant metadata.
/// - `facade`: Runtime facade path used in generated references.
/// - `integer_repr`: Integer representation spelling for fieldless enums, when
///   known.
/// - `thread_safe`: Whether thread-safe field adapters are attached.
///
/// # Returns
///
/// Returns the generated variant descriptor expression.
#[must_use]
fn variant_descriptor(
    name: &Ident,
    self_type: &TokenStream,
    variant: &VariantIr,
    facade: &TokenStream,
    integer_repr: Option<&str>,
    thread_safe: bool,
) -> TokenStream {
    let variant_name = &variant.name;
    let variant_index = variant.index;
    let variant_rust_name = variant_name.to_string();
    let query_name = variant
        .attributes
        .iter()
        .find_map(|attribute| attribute.rename())
        .unwrap_or(&variant_rust_name)
        .to_owned();
    let kind = match variant.kind {
        VariantKindIr::Unit => {
            quote!(#facade::__private::codegen_v3::descriptor::VariantKind::Unit)
        }
        VariantKindIr::Tuple => {
            quote!(#facade::__private::codegen_v3::descriptor::VariantKind::Tuple)
        }
        VariantKindIr::Struct => {
            quote!(#facade::__private::codegen_v3::descriptor::VariantKind::Struct)
        }
    };
    let origin = if variant.discriminant.is_some() {
        quote!(#facade::__private::codegen_v3::descriptor::DiscriminantOrigin::Explicit)
    } else {
        quote!(#facade::__private::codegen_v3::descriptor::DiscriminantOrigin::Implicit)
    };
    let numeric_discriminant = if variant.kind == VariantKindIr::Unit {
        numeric_discriminant(name, variant_name, integer_repr, facade)
    } else {
        quote!(None)
    };
    let active = format_ident!("__qubit_reflect_is_variant_{variant_index}");
    let fields = variant.fields.iter().map(|field| {
        let index = field.index;
        let get = format_ident!("__qubit_reflect_get_variant_{variant_index}_field_{index}");
        let get_mut = format_ident!("__qubit_reflect_get_mut_variant_{variant_index}_field_{index}");
        let set = format_ident!("__qubit_reflect_set_variant_{variant_index}_field_{index}");
        let set_preflight = format_ident!("__qubit_reflect_preflight_set_variant_{variant_index}_field_{index}");
        let get_thread_safe = format_ident!("__qubit_reflect_get_variant_{variant_index}_field_{index}_thread_safe");
        let get_mut_thread_safe = format_ident!("__qubit_reflect_get_mut_variant_{variant_index}_field_{index}_thread_safe");
        let set_thread_safe = format_ident!("__qubit_reflect_set_variant_{variant_index}_field_{index}_thread_safe");
        let set_preflight_thread_safe = format_ident!("__qubit_reflect_preflight_set_variant_{variant_index}_field_{index}_thread_safe");
        let field_rust_name = field.name.as_ref().map(|value| value.to_string());
        let field_rust_name = match field_rust_name { Some(value) => quote!(Some(#value)), None => quote!(None) };
        let query_name = field.name.as_ref().map(|field_name| field.attributes.iter().find_map(|attribute| attribute.rename()).unwrap_or(&field_name.to_string()).to_owned());
        let query_name = match query_name { Some(value) => quote!(Some(#value)), None => quote!(None) };
        let ty = &field.ty.tokens;
        let opaque_field = field.attributes.iter().any(|attribute| attribute.name == HelperName::Opaque);
        let policy = if field.attributes.iter().any(|attribute| attribute.name == HelperName::Skip) {
            quote!(#facade::__private::codegen_v3::access::FieldAccessPolicy::Skipped, None, None, None)
        } else if field.attributes.iter().any(|attribute| attribute.name == HelperName::ReadOnly) {
            quote!(#facade::__private::codegen_v3::access::FieldAccessPolicy::ReadOnly, Some(<#self_type>::#get), None, None)
        } else {
            quote!(#facade::__private::codegen_v3::access::FieldAccessPolicy::ReadWrite, Some(<#self_type>::#get), Some(<#self_type>::#get_mut), Some(<#self_type>::#set))
        };
        let preflight = if field.attributes.iter().any(|attribute| {
            matches!(attribute.name, HelperName::Skip | HelperName::ReadOnly)
        }) {
            quote!(None)
        } else {
            quote!(Some(<#self_type>::#set_preflight))
        };
        let thread_safe_access = if !thread_safe {
            TokenStream::new()
        } else if field.attributes.iter().any(|attribute| attribute.name == HelperName::Skip) {
            quote!(.with_thread_safe_access(None, None, None))
        } else if field.attributes.iter().any(|attribute| attribute.name == HelperName::ReadOnly) {
            quote!(.with_thread_safe_access(Some(<#self_type>::#get_thread_safe), None, None))
        } else {
            quote!(.with_thread_safe_access(
                Some(<#self_type>::#get_thread_safe),
                Some(<#self_type>::#get_mut_thread_safe),
                Some(<#self_type>::#set_thread_safe),
            ).with_thread_safe_set_preflight(Some(<#self_type>::#set_preflight_thread_safe)))
        };
        let descriptor = if opaque_field {
            quote!(#facade::__private::codegen_v3::descriptor::field(
                <#self_type as #facade::__private::codegen_v3::Reflect>::type_descriptor,
                #index,
                #field_rust_name,
                #query_name,
                ::std::boxed::Box::leak(::std::boxed::Box::new(
                    #facade::__private::codegen_v3::descriptor::TypeRef::Opaque(::std::boxed::Box::leak(
                        ::std::boxed::Box::new(#facade::__private::codegen_v3::descriptor::opaque_member::<#ty>()),
                    )),
                )),
                #facade::__private::codegen_v3::identity::Visibility::Private,
            ))
        } else {
            quote!(#facade::__private::codegen_v3::descriptor::lazy_field(
                <#self_type as #facade::__private::codegen_v3::Reflect>::type_descriptor,
                #index,
                #field_rust_name,
                #query_name,
                #facade::__private::codegen_v3::descriptor::lazy_type_ref::<#ty>(),
                #facade::__private::codegen_v3::identity::Visibility::Private,
            ))
        };
        quote!(#descriptor.with_access(#policy).with_set_preflight(#preflight) #thread_safe_access .with_variant(#variant_index, #variant_rust_name))
    });
    let construction = super::construction::variant_descriptor(variant, facade, thread_safe);
    quote! {{
        let fields = ::std::boxed::Box::leak(::std::vec![#(#fields),*].into_boxed_slice());
        #facade::__private::codegen_v3::descriptor::variant(<#self_type as #facade::__private::codegen_v3::Reflect>::type_descriptor, #variant_index, #variant_rust_name, #query_name, #kind, fields, <#self_type>::#active)
            .with_discriminant(#origin, #numeric_discriminant)
            #construction
    }}
}

/// Extracts and canonically orders all supported enum representation hints.
///
/// # Parameters
///
/// - `tokens`: Original enum item tokens, including compiler-validated
///   attributes.
///
/// # Returns
///
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

/// Emits an exact compiler-checked cast for a fieldless integer-repr variant.
///
/// # Parameters
///
/// - `enum_name`: Enum identifier used in the cast expression.
/// - `variant_name`: Fieldless variant identifier and source span.
/// - `repr`: Integer representation spelling, when supported.
/// - `facade`: Runtime facade path used by the generated discriminant value.
///
/// # Returns
///
/// Returns tokens for the numeric discriminant, or tokens containing `None`
/// when no integer representation applies.
#[must_use]
fn numeric_discriminant(
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
    quote!(Some(#facade::__private::codegen_v3::descriptor::NumericDiscriminant::#variant(#enum_name::#variant_name as #repr)))
}
