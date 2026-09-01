// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Procedural macros for `qubit-reflect`.

#![forbid(unsafe_code)]

use proc_macro::TokenStream;

mod entry;
mod expand;
mod internal;
mod ir;
mod parse;
mod validate;

use ir::MacroKind;

/// Parses and validates a `Reflect` derive declaration.
#[proc_macro_derive(Reflect, attributes(reflect))]
pub fn derive_reflect(input: TokenStream) -> TokenStream {
    entry::process_macro(MacroKind::Derive, TokenStream::new(), input)
}

/// Parses and validates a reflected trait declaration.
#[proc_macro_attribute]
pub fn reflect(attribute: TokenStream, item: TokenStream) -> TokenStream {
    entry::process_macro(MacroKind::Trait, attribute, item)
}

/// Parses and validates a reflected impl declaration.
#[proc_macro_attribute]
pub fn reflect_impl(attribute: TokenStream, item: TokenStream) -> TokenStream {
    entry::process_macro(MacroKind::Impl, attribute, item)
}
