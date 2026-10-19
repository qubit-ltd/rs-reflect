// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Procedural macros for `qubit-reflect`.

#![forbid(unsafe_code)]

mod configure;
mod entry;
mod expand;
mod internal;
mod ir;
mod macros;
mod parse;
mod validate;

use proc_macro::TokenStream;

/// Reflects a struct, enum, or trait declaration.
///
/// Put `#[reflect(skip)]` on a field, enum variant, or method to exclude it
/// completely from reflection metadata and generated adapters. The Rust item
/// remains unchanged.
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn Reflect(attribute: TokenStream, item: TokenStream) -> TokenStream {
    macros::reflect(attribute, item)
}

/// Reflects an inherent or trait implementation.
///
/// Methods marked with `#[reflect(skip)]` are omitted from reflected method
/// metadata and invocation adapters.
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn ReflectImpl(attribute: TokenStream, item: TokenStream) -> TokenStream {
    macros::reflect_impl(attribute, item)
}

/// Binds one explicitly monomorphized method to a dynamic invocation adapter.
#[proc_macro]
pub fn bind_method(input: TokenStream) -> TokenStream {
    macros::bind_method(input)
}

/// Registers one explicitly monomorphized method binding in the linked catalog.
#[proc_macro]
pub fn register_method_binding(input: TokenStream) -> TokenStream {
    macros::register_method_binding(input)
}

/// Defines an opaque identity handle for an external trait declaration.
#[proc_macro]
pub fn reflect_external_trait(input: TokenStream) -> TokenStream {
    macros::reflect_external_trait(input)
}

/// Registers a compiler-checked Rust trait-object type in linked contexts.
#[proc_macro]
pub fn register_trait_object(input: TokenStream) -> TokenStream {
    macros::register_trait_object(input)
}

#[proc_macro_derive(
    ConfiguredReflection,
    attributes(
        reflect,
        reflect_configure_header,
        reflect_configure_args,
        reflect_configure_kind,
        reflect_configure_member
    )
)]
#[doc(hidden)]
pub fn derive_configured_reflection(input: TokenStream) -> TokenStream {
    configure::process_configured(input)
}
