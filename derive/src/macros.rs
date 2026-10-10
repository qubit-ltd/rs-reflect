// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use proc_macro::TokenStream;

use crate::configure;
use crate::entry;
use crate::ir::MacroKind;

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
pub fn derive_reflect(input: TokenStream) -> TokenStream {
    entry::process_macro(MacroKind::Derive, TokenStream::new(), input)
}

/// Reflects a trait declaration and its metadata contract.
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
    configure::configure_attribute(MacroKind::Trait, attribute, item)
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
