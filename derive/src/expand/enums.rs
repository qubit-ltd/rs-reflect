// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Expansion of non-generic reflected enum declarations.

// qubit-style: allow explicit-imports

use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;

use super::enum_repr_ir::EnumReprIr;
use crate::expand::ExpansionContext;
use crate::ir::GenericKindIr;
use crate::ir::HelperName;
use crate::ir::TypeDeclarationIr;
use crate::ir::TypeDeclarationKindIr;
use crate::ir::VariantKindIr;

/// Expands an enum root, its variants, and safe active-variant field adapters.
pub(crate) fn expand(declaration: TypeDeclarationIr, context: &ExpansionContext) -> TokenStream {
    if declaration.kind != TypeDeclarationKindIr::Enum {
        return TokenStream::new();
    }
    let facade = context.facade().clone();
    let name = declaration.name.clone();
    let reflected_field_types = super::generics::reflected_field_types(&declaration);
    let transparently_reflected_parameters =
        super::generics::transparently_reflected_type_parameters(&declaration);
    let type_parameters: Vec<_> = declaration
        .generics
        .params
        .iter()
        .filter(|parameter| parameter.kind == GenericKindIr::Type)
        .map(|parameter| syn::Ident::new(&parameter.name, parameter.span))
        .collect();
    let mut generics: syn::Generics = match syn::parse2(declaration.generics.declaration.clone()) {
        Ok(generics) => generics,
        Err(_) => return TokenStream::new(),
    };
    if !declaration.generics.where_clause.is_empty() {
        let Ok(where_clause) = syn::parse2(declaration.generics.where_clause.clone()) else {
            return TokenStream::new();
        };
        generics.where_clause = Some(where_clause);
    }
    {
        let where_clause = generics.make_where_clause();
        for parameter in declaration
            .generics
            .params
            .iter()
            .filter(|parameter| parameter.kind == GenericKindIr::Lifetime)
        {
            let lifetime = syn::Lifetime::new(&format!("'{}", parameter.name), parameter.span);
            where_clause
                .predicates
                .push(syn::parse_quote!(#lifetime: 'static));
        }
        for parameter in &type_parameters {
            where_clause
                .predicates
                .push(syn::parse_quote!(#parameter: 'static));
        }
        for field_type in &reflected_field_types {
            let field_type = &field_type.tokens;
            where_clause
                .predicates
                .push(syn::parse_quote!(#field_type: #facade::__private::codegen_v2::Reflect));
        }
        for parameter in &transparently_reflected_parameters {
            where_clause
                .predicates
                .push(syn::parse_quote!(#parameter: #facade::__private::codegen_v2::Reflect));
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
    let capability_definition =
        super::structs::capabilities(&declaration, &facade, &capability_function);
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
        let generic_definition_provider =
            super::generics::definition_provider(&declaration, &facade);
        let type_definition_provider =
            super::generics::type_definition_provider(&declaration, &facade, fingerprint);
        let registration = registration(
            &facade,
            &name,
            &registration_module,
            fingerprint,
            !declaration.generics.params.is_empty(),
        );
        let descriptor = if declaration.generics.params.is_empty() {
            quote!(#facade::__private::codegen_v2::descriptor::with_capabilities(
                #facade::__private::codegen_v2::descriptor::opaque_root::<Self>(#query_name),
                #capability_resolver,
            ))
        } else {
            let generic = super::generics::concrete_descriptor(&declaration, &facade);
            let definition = super::generics::type_definition_provider_name(&declaration);
            quote!(#facade::__private::codegen_v2::descriptor::with_type_definition(
                #facade::__private::codegen_v2::descriptor::with_concrete_generic(
                    #facade::__private::codegen_v2::descriptor::with_capabilities(
                        #facade::__private::codegen_v2::descriptor::opaque_root::<Self>(#query_name),
                        #capability_resolver,
                    ),
                    ::std::boxed::Box::leak(::std::boxed::Box::new(#generic)),
                ),
                #definition,
            ))
        };
        return quote! {
            impl #impl_generics #name #type_generics #where_clause {
                #capability_definition
            }
            impl #impl_generics #facade::__private::codegen_v2::Reflect for #name #type_generics #where_clause {
                fn type_descriptor() -> &'static #facade::__private::codegen_v2::TypeDescriptor {
                    #facade::__private::codegen_v2::descriptor::intern_type::<Self>(|| {
                        #descriptor
                    })
                }
            }
            #registration
            #generic_definition_provider
            #type_definition_provider
        };
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
            #facade::__private::codegen_v2::descriptor::with_capabilities(
                #facade::__private::codegen_v2::descriptor::enum_type_with_repr::<Self>(
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
                #facade::__private::codegen_v2::descriptor::with_type_definition(
                    #facade::__private::codegen_v2::descriptor::with_concrete_generic(
                        #facade::__private::codegen_v2::descriptor::with_capabilities(
                            #facade::__private::codegen_v2::descriptor::enum_type_with_repr::<Self>(
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
    let type_definition_provider =
        super::generics::type_definition_provider(&declaration, &facade, fingerprint);
    let root_descriptor = quote! {
        impl #impl_generics #name #type_generics #where_clause {
            #capability_definition
            #(#adapters)*
            #(#construction_adapters)*
        }

        impl #impl_generics #facade::__private::codegen_v2::Reflect for #name #type_generics #where_clause {
            fn type_descriptor() -> &'static #facade::__private::codegen_v2::TypeDescriptor {
                #facade::__private::codegen_v2::descriptor::intern_type::<Self>(|| {
                    let variants = ::std::boxed::Box::leak(::std::vec![#(#variants),*].into_boxed_slice());
                    #enum_descriptor
                })
            }
        }

        #registration
        #generic_definition_provider
        #type_definition_provider
    };
    root_descriptor
}

/// Emits the static registry fragment for one concrete derived enum root.
fn registration(
    facade: &TokenStream,
    name: &syn::Ident,
    module: &syn::Ident,
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

            fn runtime_identity() -> #facade::__private::codegen_v2::registration::RuntimeIdentity {
                #facade::__private::codegen_v2::registration::RuntimeIdentity::Type(::std::any::TypeId::of::<#name>())
            }

            fn payload() -> #facade::__private::codegen_v2::registration::FragmentPayload {
                #facade::__private::codegen_v2::registration::FragmentPayload::Type(
                    <#name as #facade::__private::codegen_v2::Reflect>::type_descriptor(),
                )
            }

            #facade::__private::codegen_v2::inventory::submit! {
                #facade::__private::codegen_v2::registration::RegistrationFragment::new(
                    #facade::__private::codegen_v2::registration::FragmentKind::Type,
                    #facade::__private::codegen_v2::registration::StaticFragmentIdentity::new(
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
fn adapters(
    _name: &syn::Ident,
    variant: &crate::ir::VariantIr,
    facade: &TokenStream,
    thread_safe: bool,
) -> Vec<TokenStream> {
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
        fn #active(value: #facade::__private::codegen_v2::value::ReflectedRef<'_>)
            -> ::core::result::Result<bool, #facade::__private::codegen_v2::error::TypeMismatch>
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
            let inactive = quote!(#facade::__private::codegen_v2::access::FieldAccessError::inactive_variant(
                #facade::__private::codegen_v2::access::FieldIdentity::new_variant(
                    ::std::any::TypeId::of::<Self>(), ::std::any::type_name::<Self>(), #index, #rust_name,
                    #variant_index, #variant_name_text,
                ),
            ));
            let thread_safe_definitions = thread_safe.then(|| quote! {
                fn #get_thread_safe<'__qubit_reflect>(target: #facade::__private::codegen_v2::value::DynamicRef<'__qubit_reflect, #facade::__private::codegen_v2::value::ThreadSafe>)
                    -> ::core::result::Result<#facade::__private::codegen_v2::value::DynamicRef<'__qubit_reflect, #facade::__private::codegen_v2::value::ThreadSafe>, #facade::__private::codegen_v2::access::FieldAccessError>
                {
                    let value = target.downcast::<Self>().unwrap_or_else(|_| unreachable!("validated enum target"));
                    match value {
                        #pattern => Ok(#facade::__private::codegen_v2::value::DynamicRef::<#facade::__private::codegen_v2::value::ThreadSafe>::new(#binding)),
                        _ => Err(#inactive),
                    }
                }
                fn #get_mut_thread_safe<'__qubit_reflect>(target: #facade::__private::codegen_v2::value::DynamicMut<'__qubit_reflect, #facade::__private::codegen_v2::value::ThreadSafe>)
                    -> ::core::result::Result<#facade::__private::codegen_v2::value::DynamicMut<'__qubit_reflect, #facade::__private::codegen_v2::value::ThreadSafe>, #facade::__private::codegen_v2::access::FieldAccessError>
                {
                    let value = target.downcast::<Self>().unwrap_or_else(|_| unreachable!("validated enum target"));
                    match value {
                        #pattern => Ok(#facade::__private::codegen_v2::value::DynamicMut::<#facade::__private::codegen_v2::value::ThreadSafe>::new(#binding)),
                        _ => Err(#inactive),
                    }
                }
                fn #set_thread_safe(
                    target: #facade::__private::codegen_v2::value::DynamicMut<'_, #facade::__private::codegen_v2::value::ThreadSafe>,
                    replacement: #facade::__private::codegen_v2::value::DynamicOwned<#facade::__private::codegen_v2::value::ThreadSafe>,
                ) -> ::core::result::Result<(), #facade::__private::codegen_v2::access::FieldAccessError> {
                    let value = target.downcast::<Self>().unwrap_or_else(|_| unreachable!("validated enum target"));
                    let replacement = replacement.downcast::<#ty>()
                        .unwrap_or_else(|_| unreachable!("validated enum field value"));
                    match value { #pattern => { *#binding = replacement; Ok(()) }, _ => Err(#inactive) }
                }
                fn #set_preflight_thread_safe(
                    target: &#facade::__private::codegen_v2::value::DynamicMut<'_, #facade::__private::codegen_v2::value::ThreadSafe>,
                ) -> ::core::result::Result<(), #facade::__private::codegen_v2::access::FieldAccessError> {
                    let value = target.downcast_ref::<Self>()
                        .unwrap_or_else(|| unreachable!("validated enum target"));
                    match value { #active_pattern => Ok(()), _ => Err(#inactive) }
                }
            });
            vec![quote! {
                fn #get<'__qubit_reflect>(target: #facade::__private::codegen_v2::value::ReflectedRef<'__qubit_reflect>)
                    -> ::core::result::Result<#facade::__private::codegen_v2::value::ReflectedRef<'__qubit_reflect>, #facade::__private::codegen_v2::access::FieldAccessError>
                {
                    let value = target.downcast::<Self>().unwrap_or_else(|_| unreachable!("validated enum target"));
                    match value { #pattern => Ok(#facade::__private::codegen_v2::value::ReflectedRef::new(#binding)), _ => Err(#inactive) }
                }
                fn #get_mut<'__qubit_reflect>(target: #facade::__private::codegen_v2::value::ReflectedMut<'__qubit_reflect>)
                    -> ::core::result::Result<#facade::__private::codegen_v2::value::ReflectedMut<'__qubit_reflect>, #facade::__private::codegen_v2::access::FieldAccessError>
                {
                    let value = target.downcast::<Self>().unwrap_or_else(|_| unreachable!("validated enum target"));
                    match value { #pattern => Ok(#facade::__private::codegen_v2::value::ReflectedMut::new(#binding)), _ => Err(#inactive) }
                }
                fn #set(target: #facade::__private::codegen_v2::value::ReflectedMut<'_>, replacement: #facade::__private::codegen_v2::value::ReflectedOwned)
                    -> ::core::result::Result<(), #facade::__private::codegen_v2::access::FieldAccessError>
                {
                    let value = target.downcast::<Self>().unwrap_or_else(|_| unreachable!("validated enum target"));
                    let replacement = #facade::__private::codegen_v2::value::ReflectedOwned::downcast::<#ty>(replacement)
                        .unwrap_or_else(|_| unreachable!("validated enum field value"));
                    match value { #pattern => { *#binding = replacement; Ok(()) }, _ => Err(#inactive) }
                }
                fn #set_preflight(target: &#facade::__private::codegen_v2::value::ReflectedMut<'_>)
                    -> ::core::result::Result<(), #facade::__private::codegen_v2::access::FieldAccessError>
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
fn variant_descriptor(
    name: &syn::Ident,
    self_type: &TokenStream,
    variant: &crate::ir::VariantIr,
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
            quote!(#facade::__private::codegen_v2::descriptor::VariantKind::Unit)
        }
        VariantKindIr::Tuple => {
            quote!(#facade::__private::codegen_v2::descriptor::VariantKind::Tuple)
        }
        VariantKindIr::Struct => {
            quote!(#facade::__private::codegen_v2::descriptor::VariantKind::Struct)
        }
    };
    let origin = if variant.discriminant.is_some() {
        quote!(#facade::__private::codegen_v2::descriptor::DiscriminantOrigin::Explicit)
    } else {
        quote!(#facade::__private::codegen_v2::descriptor::DiscriminantOrigin::Implicit)
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
            quote!(#facade::__private::codegen_v2::access::FieldAccessPolicy::Skipped, None, None, None)
        } else if field.attributes.iter().any(|attribute| attribute.name == HelperName::ReadOnly) {
            quote!(#facade::__private::codegen_v2::access::FieldAccessPolicy::ReadOnly, Some(<#self_type>::#get), None, None)
        } else {
            quote!(#facade::__private::codegen_v2::access::FieldAccessPolicy::ReadWrite, Some(<#self_type>::#get), Some(<#self_type>::#get_mut), Some(<#self_type>::#set))
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
            quote!(#facade::__private::codegen_v2::descriptor::field(
                <#self_type as #facade::__private::codegen_v2::Reflect>::type_descriptor,
                #index,
                #field_rust_name,
                #query_name,
                ::std::boxed::Box::leak(::std::boxed::Box::new(
                    #facade::__private::codegen_v2::descriptor::TypeRef::Opaque(::std::boxed::Box::leak(
                        ::std::boxed::Box::new(#facade::__private::codegen_v2::descriptor::opaque_member::<#ty>()),
                    )),
                )),
                #facade::__private::codegen_v2::identity::Visibility::Private,
            ))
        } else {
            quote!(#facade::__private::codegen_v2::descriptor::lazy_field(
                <#self_type as #facade::__private::codegen_v2::Reflect>::type_descriptor,
                #index,
                #field_rust_name,
                #query_name,
                #facade::__private::codegen_v2::descriptor::lazy_type_ref::<#ty>(),
                #facade::__private::codegen_v2::identity::Visibility::Private,
            ))
        };
        quote!(#descriptor.with_access(#policy).with_set_preflight(#preflight) #thread_safe_access .with_variant(#variant_index, #variant_rust_name))
    });
    let construction = super::construction::variant_descriptor(variant, facade, thread_safe);
    quote! {{
        let fields = ::std::boxed::Box::leak(::std::vec![#(#fields),*].into_boxed_slice());
        #facade::__private::codegen_v2::descriptor::variant(<#self_type as #facade::__private::codegen_v2::Reflect>::type_descriptor, #variant_index, #variant_rust_name, #query_name, #kind, fields, <#self_type>::#active)
            .with_discriminant(#origin, #numeric_discriminant)
            #construction
    }}
}

/// Extracts and canonically orders all supported enum representation hints.
fn enum_representations(tokens: &TokenStream) -> Vec<EnumReprIr> {
    let Ok(input) = syn::parse2::<syn::DeriveInput>(tokens.clone()) else {
        return Vec::new();
    };
    let mut representations = Vec::new();
    for attribute in input
        .attrs
        .iter()
        .filter(|attribute| attribute.path().is_ident("repr"))
    {
        let syn::Meta::List(list) = &attribute.meta else {
            continue;
        };
        let Ok(values) = list.parse_args_with(
            syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
        ) else {
            continue;
        };
        representations.extend(values.iter().filter_map(parse_enum_representation));
    }
    representations.sort_unstable();
    representations.dedup();
    representations
}

/// Parses one compiler-validated `repr` component into structural metadata.
fn parse_enum_representation(meta: &syn::Meta) -> Option<EnumReprIr> {
    match meta {
        syn::Meta::Path(path) if path.is_ident("Rust") => Some(EnumReprIr::Rust),
        syn::Meta::Path(path) if path.is_ident("C") => Some(EnumReprIr::C),
        syn::Meta::Path(path) if path.is_ident("transparent") => Some(EnumReprIr::Transparent),
        syn::Meta::Path(path) if path.is_ident("i8") => Some(EnumReprIr::I8),
        syn::Meta::Path(path) if path.is_ident("i16") => Some(EnumReprIr::I16),
        syn::Meta::Path(path) if path.is_ident("i32") => Some(EnumReprIr::I32),
        syn::Meta::Path(path) if path.is_ident("i64") => Some(EnumReprIr::I64),
        syn::Meta::Path(path) if path.is_ident("i128") => Some(EnumReprIr::I128),
        syn::Meta::Path(path) if path.is_ident("isize") => Some(EnumReprIr::Isize),
        syn::Meta::Path(path) if path.is_ident("u8") => Some(EnumReprIr::U8),
        syn::Meta::Path(path) if path.is_ident("u16") => Some(EnumReprIr::U16),
        syn::Meta::Path(path) if path.is_ident("u32") => Some(EnumReprIr::U32),
        syn::Meta::Path(path) if path.is_ident("u64") => Some(EnumReprIr::U64),
        syn::Meta::Path(path) if path.is_ident("u128") => Some(EnumReprIr::U128),
        syn::Meta::Path(path) if path.is_ident("usize") => Some(EnumReprIr::Usize),
        syn::Meta::List(list) if list.path.is_ident("align") => {
            let alignment = syn::parse2::<syn::LitInt>(list.tokens.clone()).ok()?;
            alignment.base10_parse().ok().map(EnumReprIr::Align)
        }
        _ => None,
    }
}

/// Emits an exact compiler-checked cast for a fieldless integer-repr variant.
fn numeric_discriminant(
    enum_name: &syn::Ident,
    variant_name: &syn::Ident,
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
    let repr = syn::Ident::new(repr, variant_name.span());
    quote!(Some(#facade::__private::codegen_v2::descriptor::NumericDiscriminant::#variant(#enum_name::#variant_name as #repr)))
}
