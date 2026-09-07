// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Procedural macros for `qubit-reflect`.

#![forbid(unsafe_code)]

mod entry;
mod expand;
mod internal;
mod ir;
mod macros;
mod parse;
mod validate;

// qubit-style: allow all
use proc_macro::TokenStream;

/// Derives structural reflection for a struct or enum.
#[proc_macro_derive(Reflect, attributes(reflect))]
pub fn derive_reflect(input: TokenStream) -> TokenStream {
    macros::derive_reflect(input)
}

/// Reflects a trait declaration and its metadata contract.
#[proc_macro_attribute]
pub fn reflect(attribute: TokenStream, item: TokenStream) -> TokenStream {
    macros::reflect(attribute, item)
}

/// Reflects an inherent or trait implementation.
#[proc_macro_attribute]
pub fn reflect_impl(attribute: TokenStream, item: TokenStream) -> TokenStream {
    macros::reflect_impl(attribute, item)
}
