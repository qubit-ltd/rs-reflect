// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Expansion of safe struct construction and owned-update adapters.

use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;

use crate::ir::HelperName;
use crate::ir::HelperValueIr;
use crate::ir::TypeDeclarationIr;
use crate::ir::TypeDeclarationKindIr;
use crate::ir::VariantIr;
use crate::ir::VariantKindIr;

/// Emits generated local construction and update adapters for one struct.
pub(crate) fn struct_adapters(
    declaration: &TypeDeclarationIr,
    facade: &TokenStream,
) -> TokenStream {
    if declaration.kind != TypeDeclarationKindIr::Struct {
        return TokenStream::new();
    }
    let fields = &declaration.fields;
    let default_providers = fields.iter().filter_map(|field| {
        let default = field
            .attributes
            .iter()
            .find(|attribute| attribute.name == HelperName::Default)?;
        let provider = format_ident!("__qubit_reflect_default_field_{}", field.index);
        let ty = &field.ty.tokens;
        let expression = match &default.value {
            HelperValueIr::DefaultPath(None) => {
                quote!(<#ty as ::core::default::Default>::default())
            }
            HelperValueIr::DefaultPath(Some(path)) => {
                let path = &path.tokens;
                quote!(#path())
            }
            _ => unreachable!("validated default helper value"),
        };
        Some(quote! {
            fn #provider() -> #facade::__private::codegen_v1::value::DynamicOwned<#facade::__private::codegen_v1::value::Local> {
                let value: #ty = #expression;
                <#facade::__private::codegen_v1::value::DynamicOwned<#facade::__private::codegen_v1::value::Local>>::new(value)
            }
        })
    });
    let construction_fields = fields.iter().map(|field| {
        let index = syn::Index::from(field.index);
        let default = field.attributes.iter().any(|attribute| attribute.name == HelperName::Default);
        let restricted = field.attributes.iter().any(|attribute| {
            matches!(attribute.name, HelperName::Skip | HelperName::NoConstruct)
        });
        if default {
            let provider = format_ident!("__qubit_reflect_default_field_{}", field.index);
            if restricted {
                quote!(#facade::__private::codegen_v1::construct::ConstructionField::provider_only(&descriptor.fields()[#index], Self::#provider))
            } else {
                quote!(#facade::__private::codegen_v1::construct::ConstructionField::defaulted(&descriptor.fields()[#index], Self::#provider))
            }
        } else if restricted {
            quote!(#facade::__private::codegen_v1::construct::ConstructionField::unavailable(
                &descriptor.fields()[#index], #facade::__private::codegen_v1::construct::ConstructionUnavailableReason::MissingDefaultProvider,
            ))
        } else {
            quote!(#facade::__private::codegen_v1::construct::ConstructionField::required(&descriptor.fields()[#index]))
        }
    });
    let update_fields = fields.iter().map(|field| {
        let index = syn::Index::from(field.index);
        if field.attributes.iter().any(|attribute| attribute.name == HelperName::Skip) {
            quote!(#facade::__private::codegen_v1::construct::UpdateField::unavailable(
                &descriptor.fields()[#index], #facade::__private::codegen_v1::construct::ConstructionUnavailableReason::UpdateForbidden,
            ))
        } else {
            quote!(#facade::__private::codegen_v1::construct::UpdateField::allowed(&descriptor.fields()[#index]))
        }
    });
    let values = fields.iter().map(|field| {
        let ty = &field.ty.tokens;
        quote! {
            <#facade::__private::codegen_v1::value::DynamicOwned<#facade::__private::codegen_v1::value::Local>>::downcast::<#ty>(
                values.next().unwrap_or_else(|| unreachable!("validated construction value count")),
            ).unwrap_or_else(|_| unreachable!("validated construction field type"))
        }
    });
    let literal = match fields.first().and_then(|field| field.name.as_ref()) {
        Some(_) => {
            let assignments =
                declaration.fields.iter().zip(values).map(|(field, value)| {
                    let name = field.name.as_ref().expect("named struct field");
                    quote!(#name: #value)
                });
            quote!(Self { #(#assignments),* })
        }
        None if fields.is_empty() => quote!(Self),
        None => quote!(Self(#(#values),*)),
    };
    let assignments = fields.iter().map(|field| {
        let index = field.index;
        let ty = &field.ty.tokens;
        let target = match &field.name {
            Some(name) => quote!(#name),
            None => {
                let index = syn::Index::from(index);
                quote!(#index)
            }
        };
        quote! {
            #index => {
                value.#target = <#facade::__private::codegen_v1::value::DynamicOwned<#facade::__private::codegen_v1::value::Local>>::downcast::<#ty>(replacement)
                    .unwrap_or_else(|_| unreachable!("validated update field type"));
            }
        }
    });
    let construct = format_ident!("__qubit_reflect_struct_constructor");
    let construct_adapter = format_ident!("__qubit_reflect_construct_struct");
    let update = format_ident!("__qubit_reflect_struct_updater");
    let update_adapter = format_ident!("__qubit_reflect_update_struct");
    quote! {
        #(#default_providers)*

        fn #construct_adapter(input: #facade::__private::codegen_v1::construct::ValidatedConstructionInput<#facade::__private::codegen_v1::value::Local>)
            -> #facade::__private::codegen_v1::value::DynamicOwned<#facade::__private::codegen_v1::value::Local>
        {
            let mut values = input.into_values().into_vec().into_iter();
            <#facade::__private::codegen_v1::value::DynamicOwned<#facade::__private::codegen_v1::value::Local>>::new(#literal)
        }

        fn #construct() -> &'static #facade::__private::codegen_v1::construct::StructConstructor<#facade::__private::codegen_v1::value::Local> {
            let descriptor = <Self as #facade::__private::codegen_v1::Reflect>::type_descriptor();
            let fields = ::std::boxed::Box::leak(::std::vec![#(#construction_fields),*].into_boxed_slice());
            ::std::boxed::Box::leak(::std::boxed::Box::new(#facade::__private::codegen_v1::construct::StructConstructor::new(
                descriptor, fields, Self::#construct_adapter,
            )))
        }

        fn #update_adapter(input: #facade::__private::codegen_v1::construct::ValidatedUpdateInput<#facade::__private::codegen_v1::value::Local>)
            -> #facade::__private::codegen_v1::value::DynamicOwned<#facade::__private::codegen_v1::value::Local>
        {
            let (base, overrides) = input.into_parts();
            let mut value = <#facade::__private::codegen_v1::value::DynamicOwned<#facade::__private::codegen_v1::value::Local>>::downcast::<Self>(base)
                .unwrap_or_else(|_| unreachable!("validated update base type"));
            for override_value in overrides.into_vec() {
                let (index, replacement) = override_value.into_parts();
                match index {
                    #(#assignments)*
                    _ => unreachable!("validated update field index"),
                }
            }
            <#facade::__private::codegen_v1::value::DynamicOwned<#facade::__private::codegen_v1::value::Local>>::new(value)
        }

        fn #update() -> &'static #facade::__private::codegen_v1::construct::StructUpdater<#facade::__private::codegen_v1::value::Local> {
            let descriptor = <Self as #facade::__private::codegen_v1::Reflect>::type_descriptor();
            let fields = ::std::boxed::Box::leak(::std::vec![#(#update_fields),*].into_boxed_slice());
            ::std::boxed::Box::leak(::std::boxed::Box::new(#facade::__private::codegen_v1::construct::StructUpdater::new(
                descriptor, fields, Self::#update_adapter,
            )))
        }
    }
}

/// Returns the descriptor expression linking a struct to its generated entry
/// points.
pub(crate) fn struct_descriptor(facade: &TokenStream) -> TokenStream {
    quote!(#facade::__private::codegen_v1::construct::StructConstructionDescriptor::new(
        Self::__qubit_reflect_struct_constructor,
        Some(Self::__qubit_reflect_struct_updater),
    ))
}

/// Emits a local constructor and its descriptor attachment for one enum
/// variant.
pub(crate) fn variant_adapters(
    variant: &VariantIr,
    facade: &TokenStream,
) -> TokenStream {
    if variant.attributes.iter().any(|attribute| {
        matches!(attribute.name, HelperName::Skip | HelperName::NoConstruct)
    }) {
        return TokenStream::new();
    }
    let variant_index = variant.index;
    let constructor =
        format_ident!("__qubit_reflect_variant_constructor_{variant_index}");
    let adapter =
        format_ident!("__qubit_reflect_construct_variant_{variant_index}");
    let default_providers = variant.fields.iter().filter_map(|field| {
        let default = field
            .attributes
            .iter()
            .find(|attribute| attribute.name == HelperName::Default)?;
        let provider = format_ident!(
            "__qubit_reflect_default_variant_{variant_index}_field_{}",
            field.index
        );
        let ty = &field.ty.tokens;
        let expression = match &default.value {
            HelperValueIr::DefaultPath(None) => {
                quote!(<#ty as ::core::default::Default>::default())
            }
            HelperValueIr::DefaultPath(Some(path)) => {
                let path = &path.tokens;
                quote!(#path())
            }
            _ => unreachable!("validated default helper value"),
        };
        Some(quote! {
            fn #provider() -> #facade::__private::codegen_v1::value::DynamicOwned<#facade::__private::codegen_v1::value::Local> {
                let value: #ty = #expression;
                <#facade::__private::codegen_v1::value::DynamicOwned<#facade::__private::codegen_v1::value::Local>>::new(value)
            }
        })
    });
    let fields = variant.fields.iter();
    let policies = fields.clone().map(|field| {
        let index = syn::Index::from(field.index);
        let default = field.attributes.iter().any(|attribute| attribute.name == HelperName::Default);
        let restricted = field.attributes.iter().any(|attribute| {
            matches!(attribute.name, HelperName::Skip | HelperName::NoConstruct)
        });
        if default {
            let provider = format_ident!("__qubit_reflect_default_variant_{variant_index}_field_{}", field.index);
            if restricted {
                quote!(#facade::__private::codegen_v1::construct::ConstructionField::provider_only(&variant.fields()[#index], Self::#provider))
            } else {
                quote!(#facade::__private::codegen_v1::construct::ConstructionField::defaulted(&variant.fields()[#index], Self::#provider))
            }
        } else if restricted {
            quote!(#facade::__private::codegen_v1::construct::ConstructionField::unavailable(
                &variant.fields()[#index], #facade::__private::codegen_v1::construct::ConstructionUnavailableReason::MissingDefaultProvider,
            ))
        } else {
            quote!(#facade::__private::codegen_v1::construct::ConstructionField::required(&variant.fields()[#index]))
        }
    });
    let values = fields.clone().map(|field| {
        let ty = &field.ty.tokens;
        quote!(
            <#facade::__private::codegen_v1::value::DynamicOwned<#facade::__private::codegen_v1::value::Local>>::downcast::<#ty>(
                values.next().unwrap_or_else(|| unreachable!("validated variant value count")),
            ).unwrap_or_else(|_| unreachable!("validated variant field type"))
        )
    });
    let variant_name = &variant.name;
    let literal = match variant.kind {
        VariantKindIr::Unit => quote!(Self::#variant_name),
        VariantKindIr::Tuple => quote!(Self::#variant_name(#(#values),*)),
        VariantKindIr::Struct => {
            let assignments =
                variant.fields.iter().zip(values).map(|(field, value)| {
                    let name =
                        field.name.as_ref().expect("named variant field");
                    quote!(#name: #value)
                });
            quote!(Self::#variant_name { #(#assignments),* })
        }
    };
    quote! {
        #(#default_providers)*

        fn #adapter(input: #facade::__private::codegen_v1::construct::ValidatedConstructionInput<#facade::__private::codegen_v1::value::Local>)
            -> #facade::__private::codegen_v1::value::DynamicOwned<#facade::__private::codegen_v1::value::Local>
        {
            let mut values = input.into_values().into_vec().into_iter();
            <#facade::__private::codegen_v1::value::DynamicOwned<#facade::__private::codegen_v1::value::Local>>::new(#literal)
        }

        fn #constructor() -> &'static #facade::__private::codegen_v1::construct::VariantConstructor<#facade::__private::codegen_v1::value::Local> {
            let descriptor = <Self as #facade::__private::codegen_v1::Reflect>::type_descriptor();
            let variant = &descriptor.variants()[#variant_index];
            let fields = ::std::boxed::Box::leak(::std::vec![#(#policies),*].into_boxed_slice());
            ::std::boxed::Box::leak(::std::boxed::Box::new(#facade::__private::codegen_v1::construct::VariantConstructor::new(
                variant, fields, Self::#adapter,
            )))
        }
    }
}

/// Returns an optional descriptor attachment for one variant.
pub(crate) fn variant_descriptor(
    variant: &VariantIr,
    facade: &TokenStream,
) -> TokenStream {
    if variant.attributes.iter().any(|attribute| {
        matches!(attribute.name, HelperName::Skip | HelperName::NoConstruct)
    }) {
        return TokenStream::new();
    }
    let constructor =
        format_ident!("__qubit_reflect_variant_constructor_{}", variant.index);
    quote!(.with_construction(#facade::__private::codegen_v1::construct::VariantConstructionDescriptor::new(Self::#constructor)))
}
