// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Expansion of reflected struct declarations.

use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;

use crate::expand::ExpansionContext;
use crate::ir::GenericKindIr;
use crate::ir::HelperName;
use crate::ir::TypeDeclarationIr;
use crate::ir::TypeDeclarationKindIr;
use crate::ir::VisibilityIr;

/// Expands a concrete struct into a static root descriptor and safe field
/// adapters.
pub(crate) fn expand(declaration: TypeDeclarationIr, context: &ExpansionContext) -> TokenStream {
    if declaration.kind != TypeDeclarationKindIr::Struct {
        return TokenStream::new();
    }
    let facade = context.facade().clone();
    let name = declaration.name.clone();
    let fingerprint = context.fingerprint(&declaration.retained_tokens.to_string());
    let registration_module = format_ident!("__qubit_reflect_type_registration_{fingerprint:016x}");
    let opaque_root = declaration
        .attributes
        .iter()
        .any(|attribute| attribute.name == HelperName::Opaque);
    let reflected_field_types = super::generics::reflected_field_types(&declaration);
    let transparently_reflected_parameters = super::generics::transparently_reflected_type_parameters(&declaration);
    let type_parameter_names: Vec<_> = declaration
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
            where_clause.predicates.push(syn::parse_quote!(#lifetime: 'static));
        }
        for parameter in &type_parameter_names {
            where_clause.predicates.push(syn::parse_quote!(#parameter: 'static));
        }
        for field_type in &reflected_field_types {
            let field_type = &field_type.tokens;
            where_clause
                .predicates
                .push(syn::parse_quote!(#field_type: #facade::Reflect));
        }
        for parameter in &transparently_reflected_parameters {
            where_clause
                .predicates
                .push(syn::parse_quote!(#parameter: #facade::Reflect));
        }
    }
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let self_type = quote!(#name #type_generics);
    let query_name = declaration
        .attributes
        .iter()
        .find_map(|attribute| attribute.rename())
        .unwrap_or(&name.to_string())
        .to_owned();
    let capability_function = format_ident!("__qubit_reflect_capabilities_{fingerprint:016x}");
    let capability_resolver = quote!(<#self_type>::#capability_function);
    let capability_definition = capabilities(&declaration, &facade, &self_type, &capability_function);
    let adapter_definitions: Vec<_> = if opaque_root {
        Vec::new()
    } else {
        declaration.fields.iter().map(|field| {
        let index = field.index;
        let get = format_ident!("__qubit_reflect_get_field_{index}");
        let get_mut = format_ident!("__qubit_reflect_get_mut_field_{index}");
        let set = format_ident!("__qubit_reflect_set_field_{index}");
        let ty = &field.ty.tokens;
        let access = match &field.name {
            Some(name) => quote!(#name),
            None => {
                let index = syn::Index::from(index);
                quote!(#index)
            }
        };
        quote! {
            fn #get<'__qubit_reflect>(target: #facade::value::ReflectedRef<'__qubit_reflect>)
                -> ::core::result::Result<#facade::value::ReflectedRef<'__qubit_reflect>, #facade::access::FieldAccessError>
            {
                let value = target.downcast::<#self_type>()
                    .unwrap_or_else(|_| unreachable!("descriptor validated derived field target"));
                Ok(#facade::value::ReflectedRef::new(&value.#access))
            }

            fn #get_mut<'__qubit_reflect>(target: #facade::value::ReflectedMut<'__qubit_reflect>)
                -> ::core::result::Result<#facade::value::ReflectedMut<'__qubit_reflect>, #facade::access::FieldAccessError>
            {
                let value = target.downcast::<#self_type>()
                    .unwrap_or_else(|_| unreachable!("descriptor validated derived field target"));
                Ok(#facade::value::ReflectedMut::new(&mut value.#access))
            }

            fn #set(target: #facade::value::ReflectedMut<'_>, replacement: #facade::value::ReflectedOwned)
                -> ::core::result::Result<(), #facade::access::FieldAccessError>
            {
                let value = target.downcast::<#self_type>()
                    .unwrap_or_else(|_| unreachable!("descriptor validated derived field target"));
                let replacement = #facade::value::ReflectedOwned::downcast::<#ty>(replacement)
                    .unwrap_or_else(|_| unreachable!("descriptor validated derived field value"));
                value.#access = replacement;
                Ok(())
            }
        }
        }).collect()
    };
    let construction_adapters = if opaque_root {
        TokenStream::new()
    } else {
        super::construction::struct_adapters(&declaration, &facade)
    };
    let construction_descriptor = super::construction::struct_descriptor(&facade);
    let fields: Vec<_> = if opaque_root {
        Vec::new()
    } else {
        declaration.fields.iter().map(|field| {
        let index = field.index;
        let get = format_ident!("__qubit_reflect_get_field_{index}");
        let get_mut = format_ident!("__qubit_reflect_get_mut_field_{index}");
        let set = format_ident!("__qubit_reflect_set_field_{index}");
        let rust_name = field.name.as_ref().map(|name| name.to_string());
        let rust_name = match rust_name { Some(name) => quote!(Some(#name)), None => quote!(None) };
        let query_name = field.name.as_ref().map(|name| {
            field.attributes.iter().find_map(|attribute| attribute.rename()).unwrap_or(&name.to_string()).to_owned()
        });
        let query_name = match query_name { Some(name) => quote!(Some(#name)), None => quote!(None) };
        let ty = &field.ty.tokens;
        let opaque_field = field.attributes.iter().any(|attribute| attribute.name == HelperName::Opaque);
        let policy = if field.attributes.iter().any(|attribute| attribute.name == HelperName::Skip) {
            quote!(#facade::access::FieldAccessPolicy::Skipped, None, None, None)
        } else if field.attributes.iter().any(|attribute| attribute.name == HelperName::ReadOnly) {
            quote!(#facade::access::FieldAccessPolicy::ReadOnly, Some(<#self_type>::#get), None, None)
        } else {
            quote!(#facade::access::FieldAccessPolicy::ReadWrite, Some(<#self_type>::#get), Some(<#self_type>::#get_mut), Some(<#self_type>::#set))
        };
        let visibility = visibility(&field.visibility, &facade, field.span);
        let descriptor = if opaque_field {
            quote! {
                #facade::__private::codegen_v1::descriptor::field(
                    <#self_type as #facade::Reflect>::type_descriptor,
                    #index, #rust_name, #query_name,
                    ::std::boxed::Box::leak(::std::boxed::Box::new(
                        #facade::descriptor::TypeRef::Opaque(::std::boxed::Box::leak(
                            ::std::boxed::Box::new(#facade::__private::codegen_v1::descriptor::opaque_member::<#ty>()),
                        )),
                    )),
                    #visibility,
                )
            }
        } else {
            quote! {
                #facade::__private::codegen_v1::descriptor::lazy_field(
                    <#self_type as #facade::Reflect>::type_descriptor,
                    #index, #rust_name, #query_name,
                    #facade::__private::codegen_v1::descriptor::lazy_type_ref::<#ty>(),
                    #visibility,
                )
            }
        };
        quote! {
            #descriptor.with_access(#policy)
        }
        }).collect()
    };
    let struct_kind = match declaration.fields.len() {
        0 => quote!(#facade::descriptor::StructKind::Unit),
        1 if declaration.fields[0].name.is_none() => {
            quote!(#facade::descriptor::StructKind::Newtype)
        }
        _ if declaration.fields.first().is_some_and(|field| field.name.is_none()) => {
            quote!(#facade::descriptor::StructKind::Tuple)
        }
        _ => quote!(#facade::descriptor::StructKind::Named),
    };
    let root_descriptor = if opaque_root {
        quote!(#facade::__private::codegen_v1::descriptor::with_capabilities(
            #facade::__private::codegen_v1::descriptor::opaque_root::<Self>(#query_name),
            #capability_resolver,
        ))
    } else {
        quote! {
            let fields = ::std::boxed::Box::leak(::std::vec![#(#fields),*].into_boxed_slice());
            #facade::__private::codegen_v1::descriptor::with_capabilities(
                #facade::__private::codegen_v1::descriptor::struct_type_with_construction::<Self>(
                    #query_name, #struct_kind, fields, #construction_descriptor,
                ),
                #capability_resolver,
            )
        }
    };
    let descriptor = if declaration.generics.params.is_empty() {
        root_descriptor
    } else {
        let generic = super::generics::concrete_descriptor(&declaration, &facade);
        quote!(#facade::__private::codegen_v1::descriptor::with_concrete_generic({ #root_descriptor },
            ::std::boxed::Box::leak(::std::boxed::Box::new(#generic))))
    };
    let registration = registration(
        &facade,
        &name,
        &registration_module,
        fingerprint,
        !declaration.generics.params.is_empty(),
    );
    let generic_definition_provider = super::generics::definition_provider(&declaration, &facade);
    quote! {
        impl #impl_generics #name #type_generics #where_clause {
            #capability_definition
            #(#adapter_definitions)*
            #construction_adapters
        }

        impl #impl_generics #facade::Reflect for #name #type_generics #where_clause {
            fn type_descriptor() -> &'static #facade::TypeDescriptor {
                #facade::__private::codegen_v1::descriptor::intern_type::<Self>(|| { #descriptor })
            }
        }

        #registration
        #generic_definition_provider
    }
}

/// Expands the static capability set requested on one derived type.
pub(crate) fn capabilities(
    declaration: &TypeDeclarationIr,
    facade: &TokenStream,
    self_type: &TokenStream,
    function: &syn::Ident,
) -> TokenStream {
    let descriptors = declaration.attributes.iter().flat_map(|attribute| {
        (attribute.name == HelperName::Capabilities)
            .then_some(&attribute.value)
            .and_then(|value| match value {
                crate::ir::HelperValueIr::Paths(paths) => Some(paths),
                _ => None,
            })
            .into_iter()
            .flatten()
            .map(|path| match path.source.rsplit("::").next() {
                Some("Clone") => {
                    quote!(#facade::capability::clone_descriptor::<Self>())
                }
                Some("Default") => {
                    quote!(#facade::capability::default_descriptor::<Self>())
                }
                Some("Send") => {
                    quote!(#facade::capability::send_descriptor::<Self>())
                }
                Some("Sync") => {
                    quote!(#facade::capability::sync_descriptor::<Self>())
                }
                Some(_) | None => {
                    let tokens = &path.tokens;
                    quote!(#tokens::<Self>())
                }
            })
    });
    quote! {
        fn #function() -> &'static #facade::capability::TypeCapabilities {
            static CAPABILITIES: ::std::sync::OnceLock<#facade::capability::TypeCapabilities> =
                ::std::sync::OnceLock::new();
            CAPABILITIES.get_or_init(|| {
                let mut descriptors = ::std::vec![#(#descriptors),*];
                let registered = #facade::capability::registered_type_capabilities::<#self_type>()
                    .expect("generated capability registration is consistent");
                descriptors.extend(registered.descriptors().iter().cloned());
                #facade::capability::TypeCapabilities::try_new(descriptors)
                    .expect("generated capability declarations are unique")
            })
        }
    }
}

/// Generates a static fragment for concrete derived roots.
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
            use #facade as __qubit_reflect;

            fn runtime_identity() -> __qubit_reflect::__private::codegen_v1::registration::RuntimeIdentity {
                __qubit_reflect::__private::codegen_v1::registration::RuntimeIdentity::Type(::std::any::TypeId::of::<#name>())
            }

            fn payload() -> __qubit_reflect::__private::codegen_v1::registration::FragmentPayload {
                __qubit_reflect::__private::codegen_v1::registration::FragmentPayload::Type(
                    <#name as __qubit_reflect::Reflect>::type_descriptor(),
                )
            }

            __qubit_reflect::__private::codegen_v1::inventory::submit! {
                __qubit_reflect::__private::codegen_v1::registration::RegistrationFragment::new(
                    __qubit_reflect::__private::codegen_v1::registration::FragmentKind::Type,
                    __qubit_reflect::__private::codegen_v1::registration::StaticFragmentIdentity::new(
                        env!("CARGO_PKG_NAME"), module_path!(), line!(), column!(), "type", #fingerprint,
                    ),
                    runtime_identity,
                    payload,
                )
            }
        }
    }
}

/// Expands a normalized source visibility into its public runtime form.
fn visibility(visibility: &VisibilityIr, facade: &TokenStream, span: proc_macro2::Span) -> TokenStream {
    match visibility {
        VisibilityIr::Public => quote!(#facade::identity::Visibility::Public),
        VisibilityIr::Crate => quote!(#facade::identity::Visibility::Crate),
        VisibilityIr::Super => quote!(#facade::identity::Visibility::Super),
        VisibilityIr::SelfValue | VisibilityIr::Inherited => {
            quote!(#facade::identity::Visibility::Private)
        }
        VisibilityIr::Restricted(path) => {
            let path = syn::LitStr::new(&path.source, span);
            quote!(#facade::identity::Visibility::Restricted(#path.into()))
        }
    }
}
