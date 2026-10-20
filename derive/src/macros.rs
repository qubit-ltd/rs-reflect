// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;
use quote::format_ident;
use quote::quote;
use syn::Item;
use syn::ItemEnum;
use syn::ItemStruct;
use syn::Token;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::parse2;
use syn::spanned::Spanned;

use crate::configure;
use crate::entry;
use crate::ir::MacroKind;

/// Parses and expands a call-site generic method binding.
pub fn bind_method(input: TokenStream) -> TokenStream {
    match crate::expand::bind_method::expand(input.into()) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

/// Parses and expands a linked method binding registration.
pub fn register_method_binding(input: TokenStream) -> TokenStream {
    match crate::expand::bind_method::expand_registered(input.into()) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

struct ExternalTraitInput {
    visibility: syn::Visibility,
    marker: syn::Ident,
    path: syn::Path,
}

impl Parse for ExternalTraitInput {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let visibility = input.parse()?;
        let marker = input.parse()?;
        input.parse::<Token![=]>()?;
        let path = input.parse()?;
        input.parse::<Token![;]>()?;
        if !input.is_empty() {
            return Err(input.error("unexpected tokens after external trait declaration"));
        }
        Ok(Self {
            visibility,
            marker,
            path,
        })
    }
}

/// Expands a typed marker and linked identity registration for an external
/// trait.
pub fn reflect_external_trait(input: TokenStream) -> TokenStream {
    let result = crate::expand::ExpansionContext::from_attributes(&[])
        .and_then(|context| expand_reflect_external_trait(input.into(), &context));
    match result {
        Ok(tokens) => tokens.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

fn expand_reflect_external_trait(
    input: TokenStream2,
    context: &crate::expand::ExpansionContext,
) -> syn::Result<TokenStream2> {
    (|| {
        let input: ExternalTraitInput = parse2(input)?;
        let facade = context.facade();
        let ExternalTraitInput {
            visibility,
            marker,
            path,
        } = input;
        let source_path = path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        let source_path = if path.leading_colon.is_some() {
            format!("::{source_path}")
        } else {
            source_path
        };
        let trait_name = path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
            .ok_or_else(|| syn::Error::new(path.span(), "expected an external trait path"))?;
        let trait_name = syn::LitStr::new(&trait_name, path.span());
        let source = syn::LitStr::new(&source_path, path.span());
        let fingerprint = context.fingerprint(&source_path);
        let marker_provider = format_ident!("__qubit_reflect_external_trait_provider_{fingerprint:016x}");
        Ok(quote! {
            const _: () = #facade::__private::codegen_v4::protocol::assert_protocol_version(4);

            #[doc(hidden)]
            #visibility struct #marker;

            impl #marker {
                #[doc(hidden)]
                pub fn __qubit_reflect_trait_definition()
                    -> &'static #facade::__private::codegen_v4::traits::TraitDefinitionSpec<Self>
                {
                    #marker_provider()
                }
            }

            #[doc(hidden)]
            fn #marker_provider()
                -> &'static #facade::__private::codegen_v4::traits::TraitDefinitionSpec<#marker>
            {
                static VALUE: ::std::sync::OnceLock<
                    #facade::__private::codegen_v4::traits::TraitDefinitionSpec<#marker>
                > = ::std::sync::OnceLock::new();
                VALUE.get_or_init(||
                    #facade::__private::codegen_v4::traits::external_trait_definition::<#marker>(
                        #trait_name, #source,
                    )
                )
            }

            const _: () = {
                fn registration() -> #facade::__private::codegen_v4::registration::Registration {
                    #facade::__private::codegen_v4::traits::register_trait_definition_from::<#marker>(
                        #marker_provider,
                    )
                }
                #facade::__private::__inventory::submit! {
                    #facade::__private::codegen_v4::traits::trait_definition_fragment(
                        concat!(module_path!(), "::", stringify!(#marker)), registration,
                    )
                }
            };
        })
    })()
}

/// Expands a compiler-checked trait-object registration fragment.
pub fn register_trait_object(input: TokenStream) -> TokenStream {
    let result = crate::expand::ExpansionContext::from_attributes(&[])
        .and_then(|context| expand_register_trait_object(input.into(), &context));
    match result {
        Ok(tokens) => tokens.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

fn expand_register_trait_object(
    input: TokenStream2,
    context: &crate::expand::ExpansionContext,
) -> syn::Result<TokenStream2> {
    (|| {
        let object: syn::Type = parse2(input)?;
        if !matches!(object, syn::Type::TraitObject(_)) {
            return Err(syn::Error::new_spanned(
                object,
                "expected a trait object such as `dyn Trait`",
            ));
        }
        let facade = context.facade();
        Ok(quote! {
            const _: () = #facade::__private::codegen_v4::protocol::assert_protocol_version(4);

            const _: () = {
                fn registration() -> #facade::__private::codegen_v4::registration::Registration {
                    #facade::__private::codegen_v4::registration::Registration::metadata_only()
                        .with_trait_object(
                            #facade::__private::codegen_v4::registration::TraitObjectRegistration::of::<#object>(
                                stringify!(#object),
                            ),
                        )
                }
                #facade::__private::__inventory::submit! {
                    #facade::__private::codegen_v4::registration::trait_object_fragment(
                        concat!(module_path!(), "::", stringify!(#object)), registration,
                    )
                }
            };
        })
    })()
}

/// Derives structural reflection for a struct or enum.
///
/// Public usage and helper scopes are documented on the crate-root macro.
/// This entry lowers derive input through the shared expansion pipeline.
///
/// # Parameters
///
/// - `input`: Rust item tokens supplied to the derive macro.
///
/// # Returns
///
/// Returns generated reflection items or compiler diagnostics.
/// Reflects a struct, enum, or trait declaration.
///
/// The macro records methods, associated types, associated constants, generic
/// parameters, and supported supertraits. Dynamic invocation is emitted only
/// when the method signature can cross the reflection boundary safely;
/// unsupported methods retain a structured unavailable reason.
///
/// # Parameters
///
/// - `attribute`: Reflection macro arguments.
/// - `item`: Trait declaration tokens.
///
/// # Returns
///
/// Returns the augmented trait and generated support items or diagnostics.
pub fn reflect(attribute: TokenStream, item: TokenStream) -> TokenStream {
    let parsed = match parse2::<Item>(item.clone().into()) {
        Ok(parsed) => parsed,
        Err(error) => return error.into_compile_error().into(),
    };
    match parsed {
        Item::Struct(item) => reflect_type(attribute, item),
        Item::Enum(item) => reflect_enum(attribute, item),
        Item::Trait(_) => configure::configure_attribute(MacroKind::Trait, attribute, item),
        other => syn::Error::new_spanned(other, "`#[Reflect]` supports structs, enums, and traits")
            .into_compile_error()
            .into(),
    }
}

/// Reflects an inherent or trait implementation.
///
/// Public usage and helper scopes are documented on the crate-root macro.
/// This entry configures the internal carrier before impl expansion.
///
/// # Parameters
///
/// - `attribute`: Reflection macro arguments.
/// - `item`: Impl declaration tokens.
///
/// # Returns
///
/// Returns generated registration and invocation items or diagnostics.
pub fn reflect_impl(attribute: TokenStream, item: TokenStream) -> TokenStream {
    configure::configure_attribute(MacroKind::Impl, attribute, item)
}

fn reflect_type(attribute: TokenStream, mut item: ItemStruct) -> TokenStream {
    if !attribute.is_empty() {
        return syn::Error::new_spanned(
            TokenStream2::from(attribute),
            "`#[Reflect]` does not accept arguments on structs",
        )
        .into_compile_error()
        .into();
    }
    let reflected_item = item.to_token_stream();
    strip_attributes(&mut item.attrs);
    for field in &mut item.fields {
        strip_attributes(&mut field.attrs);
    }
    expand_type(item, reflected_item)
}

fn reflect_enum(attribute: TokenStream, mut item: ItemEnum) -> TokenStream {
    if !attribute.is_empty() {
        return syn::Error::new_spanned(
            TokenStream2::from(attribute),
            "`#[Reflect]` does not accept arguments on enums",
        )
        .into_compile_error()
        .into();
    }
    let reflected_item = item.to_token_stream();
    strip_attributes(&mut item.attrs);
    for variant in &mut item.variants {
        strip_attributes(&mut variant.attrs);
        for field in &mut variant.fields {
            strip_attributes(&mut field.attrs);
        }
    }
    expand_type(item, reflected_item)
}

fn expand_type(item: impl ToTokens, reflected_item: TokenStream2) -> TokenStream {
    let item = item.to_token_stream();
    let generated = entry::process_macro(MacroKind::Derive, TokenStream::new(), reflected_item.into());
    let item: TokenStream2 = item;
    let generated: TokenStream2 = generated.into();
    quote!(#item #generated).into()
}

fn strip_attributes(attributes: &mut Vec<syn::Attribute>) {
    attributes.retain(|attribute| !attribute.path().is_ident("reflect"));
}

#[cfg(test)]
mod tests {
    use proc_macro2::TokenStream;
    use quote::quote;

    use super::expand_reflect_external_trait;
    use super::expand_register_trait_object;
    use crate::expand::ExpansionContext;

    fn assert_v4_protocol_handshake(tokens: TokenStream) {
        let source = tokens.to_string();
        assert!(
            source.contains("codegen_v4 :: protocol :: assert_protocol_version (4)"),
            "expected the generated output to assert codegen v4, got: {source}",
        );
    }

    #[test]
    fn test_bind_method_emits_v4_protocol_handshake() {
        let context = ExpansionContext::for_tests();
        let tokens =
            crate::expand::bind_method::expand_with_context(quote!(Fixture::method as fn(&Fixture) -> u32), &context)
                .expect("method binding should expand");

        assert_v4_protocol_handshake(tokens);
    }

    #[test]
    fn test_register_method_binding_emits_v4_protocol_handshake() {
        let context = ExpansionContext::for_tests();
        let tokens = crate::expand::bind_method::expand_registered_with_context(
            quote!(Fixture::method as fn(&Fixture) -> u32),
            &context,
        )
        .expect("registered method binding should expand");

        assert_v4_protocol_handshake(tokens);
    }

    #[test]
    fn test_reflect_external_trait_emits_v4_protocol_handshake() {
        let context = ExpansionContext::for_tests();
        let tokens = expand_reflect_external_trait(quote!(pub ExternalDebug = ::std::fmt::Debug;), &context)
            .expect("external trait marker should expand");

        assert_v4_protocol_handshake(tokens);
    }

    #[test]
    fn test_register_trait_object_emits_v4_protocol_handshake() {
        let context = ExpansionContext::for_tests();
        let tokens = expand_register_trait_object(quote!(dyn ::std::fmt::Debug), &context)
            .expect("trait object registration should expand");

        assert_v4_protocol_handshake(tokens);
    }
}
