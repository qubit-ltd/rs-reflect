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
use quote::quote;
use syn::Item;
use syn::ItemEnum;
use syn::ItemStruct;
use syn::parse2;

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
