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
use syn::Index;

use crate::ir::FieldShapeIr;
use crate::ir::HelperName;
use crate::ir::HelperValueIr;
use crate::ir::TypeDeclarationIr;
use crate::ir::TypeDeclarationKindIr;
use crate::ir::VariantIr;
use crate::ir::VariantKindIr;

/// Emits generated local construction and update adapters for one struct.
///
/// # Parameters
///
/// - `declaration`: Validated struct declaration and helper attributes.
/// - `facade`: Runtime crate path used in generated references.
///
/// # Returns
///
/// Returns generated local adapters.
#[must_use]
pub(crate) fn struct_adapters(
    declaration: &TypeDeclarationIr,
    facade: &TokenStream,
) -> TokenStream {
    if super::generics::has_skipped_field(declaration, None) {
        return TokenStream::new();
    }
    struct_local_adapters(declaration, facade)
}

/// Emits the local constructor and updater implementation.
///
/// # Parameters
///
/// - `declaration`: Validated struct declaration.
/// - `facade`: Runtime crate path used in generated references.
///
/// # Returns
///
/// Returns generated items, or an empty stream for non-structs.
#[must_use]
fn struct_local_adapters(declaration: &TypeDeclarationIr, facade: &TokenStream) -> TokenStream {
    if declaration.kind != TypeDeclarationKindIr::Struct {
        return TokenStream::new();
    }
    let fields = &declaration.fields;
    let mode = quote!(#facade::__private::codegen_v3::value::Local);
    let default_providers = fields.iter().filter_map(|field| {
        let cfg_attributes = super::generics::field_cfg_attributes(declaration, None, field.index);
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
            #(#cfg_attributes)*
            fn #provider() -> #facade::__private::codegen_v3::value::DynamicOwned<#mode> {
                let value: #ty = #expression;
                <#facade::__private::codegen_v3::value::DynamicOwned<#mode>>::new(value)
            }
        })
    });
    let construction_fields = fields.iter().map(|field| {
        let cfg_attributes = super::generics::field_cfg_attributes(declaration, None, field.index);
        let index = Index::from(field.index);
        let default = field
            .attributes
            .iter()
            .any(|attribute| attribute.name == HelperName::Default);
        let restricted = field
            .attributes
            .iter()
            .any(|attribute| matches!(attribute.name, HelperName::Skip | HelperName::NoConstruct));
        let construction_field = if default {
            let provider = format_ident!("__qubit_reflect_default_field_{}", field.index);
            if restricted {
                quote!(#facade::__private::codegen_v3::construct::ConstructionField::provider_only(
                    field, Self::#provider,
                ))
            } else {
                quote!(#facade::__private::codegen_v3::construct::ConstructionField::defaulted(
                    field, Self::#provider,
                ))
            }
        } else if restricted {
            quote!(#facade::__private::codegen_v3::construct::ConstructionField::unavailable(
                field,
                #facade::__private::codegen_v3::construct::ConstructionUnavailableReason::MissingDefaultProvider,
            ))
        } else {
            quote!(#facade::__private::codegen_v3::construct::ConstructionField::required(field))
        };
        quote!(#(#cfg_attributes)* #index => #construction_field)
    });
    let update_fields = fields.iter().map(|field| {
        let cfg_attributes = super::generics::field_cfg_attributes(declaration, None, field.index);
        let index = Index::from(field.index);
        let update_field = if field
            .attributes
            .iter()
            .any(|attribute| attribute.name == HelperName::Skip)
        {
            quote!(#facade::__private::codegen_v3::construct::UpdateField::unavailable(
                field,
                #facade::__private::codegen_v3::construct::ConstructionUnavailableReason::UpdateForbidden,
            ))
        } else {
            quote!(#facade::__private::codegen_v3::construct::UpdateField::allowed(field))
        };
        quote!(#(#cfg_attributes)* #index => #update_field)
    });
    let values = fields.iter().map(|field| {
        let cfg_attributes = super::generics::field_cfg_attributes(declaration, None, field.index);
        let ty = &field.ty.tokens;
        quote! {
            #(#cfg_attributes)*
            <#facade::__private::codegen_v3::value::DynamicOwned<#mode>>::downcast::<#ty>(
                values.next().unwrap_or_else(|| unreachable!("validated construction value count")),
            ).unwrap_or_else(|_| unreachable!("validated construction field type"))
        }
    });
    let literal = match declaration.field_shape {
        FieldShapeIr::Named => {
            let assignments = declaration.fields.iter().zip(values).map(|(field, value)| {
                let name = field.name.as_ref().expect("named struct field");
                let cfg_attributes =
                    super::generics::field_cfg_attributes(declaration, None, field.index);
                quote!(#(#cfg_attributes)* #name: #value)
            });
            quote!(Self { #(#assignments),* })
        }
        FieldShapeIr::Unit => quote!(Self),
        FieldShapeIr::Unnamed => quote!(Self(#(#values),*)),
    };
    let assignments = fields.iter().map(|field| {
        let cfg_attributes = super::generics::field_cfg_attributes(declaration, None, field.index);
        let index = field.index;
        let ty = &field.ty.tokens;
        let target = match &field.name {
            Some(name) => quote!(#name),
            None => {
                let index = Index::from(index);
                quote!(#index)
            }
        };
        quote! {
            #(#cfg_attributes)*
            #index => {
                value.#target = <#facade::__private::codegen_v3::value::DynamicOwned<#mode>>::downcast::<#ty>(
                    replacement,
                )
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

        fn #construct_adapter(input: #facade::__private::codegen_v3::construct::ValidatedConstructionInput<#mode>)
            -> #facade::__private::codegen_v3::value::DynamicOwned<#mode>
        {
            let mut values = input.into_values().into_vec().into_iter();
            <#facade::__private::codegen_v3::value::DynamicOwned<#mode>>::new(#literal)
        }

        fn #construct() -> &'static #facade::__private::codegen_v3::construct::StructConstructor<#mode> {
            let descriptor = <Self as #facade::__private::codegen_v3::Reflect>::type_descriptor();
            let fields = descriptor.fields().iter().map(|field| match field.index() {
                #(#construction_fields,)*
                _ => unreachable!("generated construction field index"),
            }).collect::<::std::vec::Vec<_>>();
            let fields = ::std::boxed::Box::leak(fields.into_boxed_slice());
            ::std::boxed::Box::leak(::std::boxed::Box::new(
                #facade::__private::codegen_v3::construct::StructConstructor::new(
                    descriptor, fields, Self::#construct_adapter,
                ),
            ))
        }

        fn #update_adapter(input: #facade::__private::codegen_v3::construct::ValidatedUpdateInput<#mode>)
            -> #facade::__private::codegen_v3::value::DynamicOwned<#mode>
        {
            let (base, overrides) = input.into_parts();
            let mut value = <#facade::__private::codegen_v3::value::DynamicOwned<#mode>>::downcast::<Self>(base)
                .unwrap_or_else(|_| unreachable!("validated update base type"));
            for override_value in overrides.into_vec() {
                let (index, replacement) = override_value.into_parts();
                match index {
                    #(#assignments)*
                    _ => unreachable!("validated update field index"),
                }
            }
            <#facade::__private::codegen_v3::value::DynamicOwned<#mode>>::new(value)
        }

        fn #update() -> &'static #facade::__private::codegen_v3::construct::StructUpdater<#mode> {
            let descriptor = <Self as #facade::__private::codegen_v3::Reflect>::type_descriptor();
            let fields = descriptor.fields().iter().map(|field| match field.index() {
                #(#update_fields,)*
                _ => unreachable!("generated update field index"),
            }).collect::<::std::vec::Vec<_>>();
            let fields = ::std::boxed::Box::leak(fields.into_boxed_slice());
            ::std::boxed::Box::leak(::std::boxed::Box::new(
                #facade::__private::codegen_v3::construct::StructUpdater::new(
                    descriptor, fields, Self::#update_adapter,
                ),
            ))
        }
    }
}

/// Returns the descriptor expression linking a struct to its generated entry
/// points.
///
/// # Parameters
///
/// - `facade`: Runtime crate path used in the descriptor expression.
///
/// # Returns
///
/// Returns the generated construction descriptor expression.
#[must_use]
pub(crate) fn struct_descriptor(facade: &TokenStream) -> TokenStream {
    quote!(#facade::__private::codegen_v3::construct::StructConstructionDescriptor::new(
        Self::__qubit_reflect_struct_constructor,
        Some(Self::__qubit_reflect_struct_updater),
    ))
}

/// Emits a local constructor and its descriptor attachment for one enum
/// variant.
///
/// # Parameters
///
/// - `variant`: Validated enum variant declaration.
/// - `facade`: Runtime crate path used in generated references.
///
/// # Returns
///
/// Returns generated local variant adapters.
#[must_use]
pub(crate) fn variant_adapters(
    declaration: &TypeDeclarationIr,
    variant: &VariantIr,
    facade: &TokenStream,
) -> TokenStream {
    if super::generics::has_skipped_field(declaration, Some(variant.index)) {
        return TokenStream::new();
    }
    variant_local_adapter(declaration, variant, facade)
}

/// Emits one local enum variant constructor.
///
/// # Parameters
///
/// - `variant`: Validated enum variant declaration.
/// - `facade`: Runtime crate path used in generated references.
///
/// # Returns
///
/// Returns generated items, or an empty stream when construction is disabled
/// for the variant.
#[must_use]
fn variant_local_adapter(
    declaration: &TypeDeclarationIr,
    variant: &VariantIr,
    facade: &TokenStream,
) -> TokenStream {
    if variant
        .attributes
        .iter()
        .any(|attribute| matches!(attribute.name, HelperName::Skip | HelperName::NoConstruct))
    {
        return TokenStream::new();
    }
    let variant_index = variant.index;
    let variant_cfg_attributes =
        super::generics::variant_cfg_attributes(declaration, variant_index);
    let constructor = format_ident!("__qubit_reflect_variant_constructor_{variant_index}");
    let adapter = format_ident!("__qubit_reflect_construct_variant_{variant_index}");
    let mode = quote!(#facade::__private::codegen_v3::value::Local);
    let default_providers = variant.fields.iter().filter_map(|field| {
        let cfg_attributes =
            super::generics::field_cfg_attributes(declaration, Some(variant_index), field.index);
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
            #(#variant_cfg_attributes)*
            #(#cfg_attributes)*
            fn #provider() -> #facade::__private::codegen_v3::value::DynamicOwned<#mode> {
                let value: #ty = #expression;
                <#facade::__private::codegen_v3::value::DynamicOwned<#mode>>::new(value)
            }
        })
    });
    let fields = variant.fields.iter();
    let policies = fields.clone().map(|field| {
        let cfg_attributes = super::generics::field_cfg_attributes(declaration, Some(variant_index), field.index);
        let index = Index::from(field.index);
        let default = field
            .attributes
            .iter()
            .any(|attribute| attribute.name == HelperName::Default);
        let restricted = field
            .attributes
            .iter()
            .any(|attribute| matches!(attribute.name, HelperName::Skip | HelperName::NoConstruct));
        let policy = if default {
            let provider = format_ident!("__qubit_reflect_default_variant_{variant_index}_field_{}", field.index,);
            if restricted {
                quote!(#facade::__private::codegen_v3::construct::ConstructionField::provider_only(
                    field, Self::#provider,
                ))
            } else {
                quote!(#facade::__private::codegen_v3::construct::ConstructionField::defaulted(
                    field, Self::#provider,
                ))
            }
        } else if restricted {
            quote!(#facade::__private::codegen_v3::construct::ConstructionField::unavailable(
                field,
                #facade::__private::codegen_v3::construct::ConstructionUnavailableReason::MissingDefaultProvider,
            ))
        } else {
            quote!(#facade::__private::codegen_v3::construct::ConstructionField::required(field))
        };
        quote!(#(#variant_cfg_attributes)* #(#cfg_attributes)* #index => #policy)
    });
    let values = fields.clone().map(|field| {
        let cfg_attributes =
            super::generics::field_cfg_attributes(declaration, Some(variant_index), field.index);
        let ty = &field.ty.tokens;
        quote!(
            #(#variant_cfg_attributes)*
            #(#cfg_attributes)*
            <#facade::__private::codegen_v3::value::DynamicOwned<#mode>>::downcast::<#ty>(
                values.next().unwrap_or_else(|| unreachable!("validated variant value count")),
            ).unwrap_or_else(|_| unreachable!("validated variant field type"))
        )
    });
    let variant_name = &variant.name;
    let literal = match variant.kind {
        VariantKindIr::Unit => quote!(Self::#variant_name),
        VariantKindIr::Tuple => quote!(Self::#variant_name(#(#values),*)),
        VariantKindIr::Struct => {
            let assignments = variant.fields.iter().zip(values).map(|(field, value)| {
                let name = field.name.as_ref().expect("named variant field");
                let cfg_attributes = super::generics::field_cfg_attributes(
                    declaration,
                    Some(variant_index),
                    field.index,
                );
                quote!(#(#variant_cfg_attributes)* #(#cfg_attributes)* #name: #value)
            });
            quote!(Self::#variant_name { #(#assignments),* })
        }
    };
    quote! {
        #(#default_providers)*

        #(#variant_cfg_attributes)*
        fn #adapter(input: #facade::__private::codegen_v3::construct::ValidatedConstructionInput<#mode>)
            -> #facade::__private::codegen_v3::value::DynamicOwned<#mode>
        {
            let mut values = input.into_values().into_vec().into_iter();
            <#facade::__private::codegen_v3::value::DynamicOwned<#mode>>::new(#literal)
        }

        #(#variant_cfg_attributes)*
        fn #constructor() -> &'static #facade::__private::codegen_v3::construct::VariantConstructor<#mode> {
            let descriptor = <Self as #facade::__private::codegen_v3::Reflect>::type_descriptor();
            let variant = descriptor
                .variant_at(#variant_index)
                .unwrap_or_else(|| unreachable!("generated variant source index must exist"));
            let fields = variant.fields().iter().map(|field| match field.index() {
                #(#policies,)*
                _ => unreachable!("generated variant field index"),
            }).collect::<::std::vec::Vec<_>>();
            let fields = ::std::boxed::Box::leak(fields.into_boxed_slice());
            ::std::boxed::Box::leak(::std::boxed::Box::new(
                #facade::__private::codegen_v3::construct::VariantConstructor::new(
                    variant, fields, Self::#adapter,
                ),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use crate::expand::ExpansionContext;
    use crate::ir::DeclarationIr;
    use crate::ir::MacroKind;
    use crate::parse::parse_declaration;

    #[test]
    fn test_struct_construction_emits_codegen_v4_only() {
        let parsed = parse_declaration(
            MacroKind::Derive,
            quote!(),
            quote! {
                struct Fixture {
                    value: u8,
                }
            },
        )
        .expect("the construction fixture must parse");
        let DeclarationIr::Type(declaration) = parsed.declaration else {
            panic!("the construction fixture must produce a type declaration");
        };
        let generated = crate::expand::structs::expand(declaration, &ExpansionContext::for_tests())
            .expect("the struct should expand")
            .to_string();

        assert!(
            generated.contains("codegen_v4"),
            "struct construction must use the v4 protocol"
        );
        assert!(
            !generated.contains("codegen_v3"),
            "struct construction must not use the v3 protocol"
        );
        assert!(
            generated.contains("ConstructProvider"),
            "struct construction must emit a typed provider"
        );
        assert!(
            generated.contains("ConstructionPlan :: for_type"),
            "struct construction must emit a plan"
        );
    }

    #[test]
    fn test_variant_construction_emits_codegen_v4_only() {
        let parsed = parse_declaration(
            MacroKind::Derive,
            quote!(),
            quote! {
                enum Fixture {
                    Value(u8),
                }
            },
        )
        .expect("the construction fixture must parse");
        let DeclarationIr::Type(declaration) = parsed.declaration else {
            panic!("the construction fixture must produce a type declaration");
        };
        let generated = crate::expand::enums::expand(declaration, &ExpansionContext::for_tests())
            .expect("the enum should expand")
            .to_string();

        assert!(
            generated.contains("codegen_v4"),
            "variant construction must use the v4 protocol"
        );
        assert!(
            !generated.contains("codegen_v3"),
            "variant construction must not use the v3 protocol"
        );
        assert!(
            generated.contains("ConstructProvider"),
            "variant construction must emit a typed provider"
        );
        assert!(
            generated.contains("ConstructionPlan :: for_variant"),
            "variant construction must emit a plan"
        );
    }
}

/// Returns an optional descriptor attachment for one variant.
///
/// # Parameters
///
/// - `variant`: Validated enum variant declaration.
/// - `facade`: Runtime crate path used in the descriptor expression.
///
/// # Returns
///
/// Returns generated descriptor tokens, or an empty stream when construction is
/// disabled.
#[must_use]
pub(crate) fn variant_descriptor(variant: &VariantIr, facade: &TokenStream) -> TokenStream {
    if variant
        .attributes
        .iter()
        .any(|attribute| matches!(attribute.name, HelperName::Skip | HelperName::NoConstruct))
    {
        return TokenStream::new();
    }
    let constructor = format_ident!("__qubit_reflect_variant_constructor_{}", variant.index);
    quote!(.with_construction(
        #facade::__private::codegen_v3::construct::VariantConstructionDescriptor::new(Self::#constructor)
    ))
}
