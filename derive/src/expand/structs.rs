// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Expansion of reflected struct declarations.

// qubit-style: allow explicit-imports

use proc_macro2::Span;
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
    let thread_safe = declaration
        .attributes
        .iter()
        .any(|attribute| attribute.name == HelperName::ThreadSafe);
    let reflected_field_types = super::generics::reflected_field_types(&declaration);
    let transparently_reflected_parameters =
        super::generics::transparently_reflected_type_parameters(&declaration);
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
            where_clause
                .predicates
                .push(syn::parse_quote!(#lifetime: 'static));
        }
        for parameter in &type_parameter_names {
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
    let query_name = declaration
        .attributes
        .iter()
        .find_map(|attribute| attribute.rename())
        .unwrap_or(&name.to_string())
        .to_owned();
    let capability_function = format_ident!("__qubit_reflect_capabilities_{fingerprint:016x}");
    let capability_resolver = quote!(<#self_type>::#capability_function);
    let capability_definition = capabilities(&declaration, &facade, &capability_function);
    let adapter_definitions: Vec<_> = if opaque_root {
        Vec::new()
    } else {
        declaration.fields.iter().map(|field| {
        let index = field.index;
        let get = format_ident!("__qubit_reflect_get_field_{index}");
        let get_mut = format_ident!("__qubit_reflect_get_mut_field_{index}");
        let set = format_ident!("__qubit_reflect_set_field_{index}");
        let get_thread_safe = format_ident!("__qubit_reflect_get_field_{index}_thread_safe");
        let get_mut_thread_safe = format_ident!("__qubit_reflect_get_mut_field_{index}_thread_safe");
        let set_thread_safe = format_ident!("__qubit_reflect_set_field_{index}_thread_safe");
        let ty = &field.ty.tokens;
        let access = match &field.name {
            Some(name) => quote!(#name),
            None => {
                let index = syn::Index::from(index);
                quote!(#index)
            }
        };
        let thread_safe_adapters = thread_safe.then(|| quote! {
            fn #get_thread_safe<'__qubit_reflect>(target: #facade::__private::codegen_v2::value::DynamicRef<'__qubit_reflect, #facade::__private::codegen_v2::value::ThreadSafe>)
                -> ::core::result::Result<#facade::__private::codegen_v2::value::DynamicRef<'__qubit_reflect, #facade::__private::codegen_v2::value::ThreadSafe>, #facade::__private::codegen_v2::access::FieldAccessError>
            {
                let value = target.downcast::<#self_type>()
                    .unwrap_or_else(|_| unreachable!("descriptor validated derived field target"));
                Ok(#facade::__private::codegen_v2::value::DynamicRef::<#facade::__private::codegen_v2::value::ThreadSafe>::new(&value.#access))
            }

            fn #get_mut_thread_safe<'__qubit_reflect>(target: #facade::__private::codegen_v2::value::DynamicMut<'__qubit_reflect, #facade::__private::codegen_v2::value::ThreadSafe>)
                -> ::core::result::Result<#facade::__private::codegen_v2::value::DynamicMut<'__qubit_reflect, #facade::__private::codegen_v2::value::ThreadSafe>, #facade::__private::codegen_v2::access::FieldAccessError>
            {
                let value = target.downcast::<#self_type>()
                    .unwrap_or_else(|_| unreachable!("descriptor validated derived field target"));
                Ok(#facade::__private::codegen_v2::value::DynamicMut::<#facade::__private::codegen_v2::value::ThreadSafe>::new(&mut value.#access))
            }

            fn #set_thread_safe(
                target: #facade::__private::codegen_v2::value::DynamicMut<'_, #facade::__private::codegen_v2::value::ThreadSafe>,
                replacement: #facade::__private::codegen_v2::value::DynamicOwned<#facade::__private::codegen_v2::value::ThreadSafe>,
            ) -> ::core::result::Result<(), #facade::__private::codegen_v2::access::FieldAccessError> {
                let value = target.downcast::<#self_type>()
                    .unwrap_or_else(|_| unreachable!("descriptor validated derived field target"));
                let replacement = replacement.downcast::<#ty>()
                    .unwrap_or_else(|_| unreachable!("descriptor validated derived field value"));
                value.#access = replacement;
                Ok(())
            }
        });
        quote! {
            fn #get<'__qubit_reflect>(target: #facade::__private::codegen_v2::value::ReflectedRef<'__qubit_reflect>)
                -> ::core::result::Result<#facade::__private::codegen_v2::value::ReflectedRef<'__qubit_reflect>, #facade::__private::codegen_v2::access::FieldAccessError>
            {
                let value = target.downcast::<#self_type>()
                    .unwrap_or_else(|_| unreachable!("descriptor validated derived field target"));
                Ok(#facade::__private::codegen_v2::value::ReflectedRef::new(&value.#access))
            }

            fn #get_mut<'__qubit_reflect>(target: #facade::__private::codegen_v2::value::ReflectedMut<'__qubit_reflect>)
                -> ::core::result::Result<#facade::__private::codegen_v2::value::ReflectedMut<'__qubit_reflect>, #facade::__private::codegen_v2::access::FieldAccessError>
            {
                let value = target.downcast::<#self_type>()
                    .unwrap_or_else(|_| unreachable!("descriptor validated derived field target"));
                Ok(#facade::__private::codegen_v2::value::ReflectedMut::new(&mut value.#access))
            }

            fn #set(target: #facade::__private::codegen_v2::value::ReflectedMut<'_>, replacement: #facade::__private::codegen_v2::value::ReflectedOwned)
                -> ::core::result::Result<(), #facade::__private::codegen_v2::access::FieldAccessError>
            {
                let value = target.downcast::<#self_type>()
                    .unwrap_or_else(|_| unreachable!("descriptor validated derived field target"));
                let replacement = #facade::__private::codegen_v2::value::ReflectedOwned::downcast::<#ty>(replacement)
                    .unwrap_or_else(|_| unreachable!("descriptor validated derived field value"));
                value.#access = replacement;
                Ok(())
            }

            #thread_safe_adapters
        }
        }).collect()
    };
    let construction_adapters = if opaque_root {
        TokenStream::new()
    } else {
        super::construction::struct_adapters(&declaration, &facade)
    };
    let construction_descriptor = super::construction::struct_descriptor(&declaration, &facade);
    let fields: Vec<_> = if opaque_root {
        Vec::new()
    } else {
        declaration.fields.iter().map(|field| {
        let index = field.index;
        let get = format_ident!("__qubit_reflect_get_field_{index}");
        let get_mut = format_ident!("__qubit_reflect_get_mut_field_{index}");
        let set = format_ident!("__qubit_reflect_set_field_{index}");
        let get_thread_safe = format_ident!("__qubit_reflect_get_field_{index}_thread_safe");
        let get_mut_thread_safe = format_ident!("__qubit_reflect_get_mut_field_{index}_thread_safe");
        let set_thread_safe = format_ident!("__qubit_reflect_set_field_{index}_thread_safe");
        let rust_name = field.name.as_ref().map(|name| name.to_string());
        let rust_name = match rust_name { Some(name) => quote!(Some(#name)), None => quote!(None) };
        let query_name = field.name.as_ref().map(|name| {
            field.attributes.iter().find_map(|attribute| attribute.rename()).unwrap_or(&name.to_string()).to_owned()
        });
        let query_name = match query_name { Some(name) => quote!(Some(#name)), None => quote!(None) };
        let ty = &field.ty.tokens;
        let opaque_field = field.attributes.iter().any(|attribute| attribute.name == HelperName::Opaque);
        let policy = if field.attributes.iter().any(|attribute| attribute.name == HelperName::Skip) {
            quote!(#facade::__private::codegen_v2::access::FieldAccessPolicy::Skipped, None, None, None)
        } else if field.attributes.iter().any(|attribute| attribute.name == HelperName::ReadOnly) {
            quote!(#facade::__private::codegen_v2::access::FieldAccessPolicy::ReadOnly, Some(<#self_type>::#get), None, None)
        } else {
            quote!(#facade::__private::codegen_v2::access::FieldAccessPolicy::ReadWrite, Some(<#self_type>::#get), Some(<#self_type>::#get_mut), Some(<#self_type>::#set))
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
            ))
        };
        let visibility = visibility(&field.visibility, &facade, field.span);
        let descriptor = if opaque_field {
            quote! {
                #facade::__private::codegen_v2::descriptor::field(
                    <#self_type as #facade::__private::codegen_v2::Reflect>::type_descriptor,
                    #index, #rust_name, #query_name,
                    ::std::boxed::Box::leak(::std::boxed::Box::new(
                        #facade::__private::codegen_v2::descriptor::TypeRef::Opaque(::std::boxed::Box::leak(
                            ::std::boxed::Box::new(#facade::__private::codegen_v2::descriptor::opaque_member::<#ty>()),
                        )),
                    )),
                    #visibility,
                )
            }
        } else {
            quote! {
                #facade::__private::codegen_v2::descriptor::lazy_field(
                    <#self_type as #facade::__private::codegen_v2::Reflect>::type_descriptor,
                    #index, #rust_name, #query_name,
                    #facade::__private::codegen_v2::descriptor::lazy_type_ref::<#ty>(),
                    #visibility,
                )
            }
        };
        quote! {
            #descriptor.with_access(#policy) #thread_safe_access
        }
        }).collect()
    };
    let struct_kind = match declaration.fields.len() {
        0 => quote!(#facade::__private::codegen_v2::descriptor::StructKind::Unit),
        1 if declaration.fields[0].name.is_none() => {
            quote!(#facade::__private::codegen_v2::descriptor::StructKind::Newtype)
        }
        _ if declaration
            .fields
            .first()
            .is_some_and(|field| field.name.is_none()) =>
        {
            quote!(#facade::__private::codegen_v2::descriptor::StructKind::Tuple)
        }
        _ => quote!(#facade::__private::codegen_v2::descriptor::StructKind::Named),
    };
    let root_descriptor = if opaque_root {
        quote!(#facade::__private::codegen_v2::descriptor::with_capabilities(
            #facade::__private::codegen_v2::descriptor::opaque_root::<Self>(#query_name),
            #capability_resolver,
        ))
    } else {
        quote! {
            let fields = ::std::boxed::Box::leak(::std::vec![#(#fields),*].into_boxed_slice());
            #facade::__private::codegen_v2::descriptor::with_capabilities(
                #facade::__private::codegen_v2::descriptor::struct_type_with_construction::<Self>(
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
        quote!(#facade::__private::codegen_v2::descriptor::with_concrete_generic({ #root_descriptor },
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
    let generic_template_provider = super::generics::template_provider(&declaration, &facade);
    quote! {
        impl #impl_generics #name #type_generics #where_clause {
            #capability_definition
            #(#adapter_definitions)*
            #construction_adapters
        }

        impl #impl_generics #facade::__private::codegen_v2::Reflect for #name #type_generics #where_clause {
            fn type_descriptor() -> &'static #facade::__private::codegen_v2::TypeDescriptor {
                #facade::__private::codegen_v2::descriptor::intern_type::<Self>(|| { #descriptor })
            }
        }

        #registration
        #generic_definition_provider
        #generic_template_provider
    }
}

/// Expands the static capability set requested on one derived type.
pub(crate) fn capabilities(
    declaration: &TypeDeclarationIr,
    facade: &TokenStream,
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
                    quote!(#facade::__private::codegen_v2::capability::clone_descriptor::<Self>())
                }
                Some("Default") => {
                    quote!(#facade::__private::codegen_v2::capability::default_descriptor::<Self>())
                }
                Some("Send") => {
                    quote!(#facade::__private::codegen_v2::capability::send_descriptor::<Self>())
                }
                Some("Sync") => {
                    quote!(#facade::__private::codegen_v2::capability::sync_descriptor::<Self>())
                }
                Some(_) | None => {
                    let tokens = &path.tokens;
                    quote!(#tokens::<Self>())
                }
            })
    });
    quote! {
        fn #function() -> &'static #facade::__private::codegen_v2::capability::TypeCapabilities {
            static CAPABILITIES: ::std::sync::OnceLock<#facade::__private::codegen_v2::capability::TypeCapabilities> =
                ::std::sync::OnceLock::new();
            CAPABILITIES.get_or_init(|| {
                #facade::__private::codegen_v2::capability::TypeCapabilities::try_new(
                    ::std::vec![#(#descriptors),*],
                )
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

/// Expands a normalized source visibility into its public runtime form.
fn visibility(visibility: &VisibilityIr, facade: &TokenStream, span: Span) -> TokenStream {
    match visibility {
        VisibilityIr::Public => {
            quote!(#facade::__private::codegen_v2::identity::Visibility::Public)
        }
        VisibilityIr::Crate => quote!(#facade::__private::codegen_v2::identity::Visibility::Crate),
        VisibilityIr::Super => quote!(#facade::__private::codegen_v2::identity::Visibility::Super),
        VisibilityIr::SelfValue | VisibilityIr::Inherited => {
            quote!(#facade::__private::codegen_v2::identity::Visibility::Private)
        }
        VisibilityIr::Restricted(path) => {
            let path = syn::LitStr::new(&path.source, span);
            quote!(#facade::__private::codegen_v2::identity::Visibility::Restricted(#path.into()))
        }
    }
}
