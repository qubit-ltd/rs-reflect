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

// qubit-style: allow all
use proc_macro::TokenStream;

/// Derives structural reflection for a struct or enum.
///
/// # Parameters
///
/// - `input`: Rust item tokens supplied to the derive macro.
///
/// # Returns
///
/// Returns generated reflection items or compiler diagnostics.
#[proc_macro_derive(Reflect, attributes(reflect))]
pub fn derive_reflect(input: TokenStream) -> TokenStream {
    macros::derive_reflect(input)
}

/// Reflects a trait declaration and its metadata contract.
///
/// Apply this macro to a trait and place `reflect` helpers on its supported
/// direct members. Rust evaluates `cfg` and `cfg_attr` before helper
/// validation, descriptor collection, adapter generation, and specialization
/// analysis. An inactive member is absent from the reflected definition.
/// `#[reflect(no_invoke)]` keeps active metadata while withholding its
/// invocation adapter. Attribute macro facades that forward this macro must
/// re-export the versioned internal derive support or enable the runtime
/// facade's `derive` feature. See the [user guide](https://github.com/qubit-ltd/rs-reflect/blob/main/doc/2026-08-29-qubit-reflect-user-guide.md)
/// for helper placement and facade examples.
///
/// # Parameters
///
/// - `attribute`: Reflection macro arguments.
/// - `item`: Trait declaration tokens.
///
/// # Returns
///
/// Returns the augmented trait and generated support items or diagnostics.
#[proc_macro_attribute]
pub fn reflect(attribute: TokenStream, item: TokenStream) -> TokenStream {
    macros::reflect(attribute, item)
}

/// Reflects an inherent or trait implementation.
///
/// Rust evaluates `cfg` and `cfg_attr` before helper validation, metadata
/// collection, adapter generation, and specialization analysis. This keeps
/// generated calls aligned with the members compiled into the impl.
/// Helper placement, the active-member rules, and the internal support
/// requirement for macro-forwarding facades are described in the [user
/// guide](https://github.com/qubit-ltd/rs-reflect/blob/main/doc/2026-08-29-qubit-reflect-user-guide.md).
///
/// # Parameters
///
/// - `attribute`: Reflection macro arguments.
/// - `item`: Impl declaration tokens.
///
/// # Returns
///
/// Returns generated registration and invocation items or diagnostics.
#[proc_macro_attribute]
pub fn reflect_impl(attribute: TokenStream, item: TokenStream) -> TokenStream {
    macros::reflect_impl(attribute, item)
}

/// Resolves conditional configuration for one reflected trait or impl.
///
/// # Parameters
///
/// - `input`: Compiler-filtered internal carrier tokens.
///
/// # Returns
///
/// Returns the reflected declaration and generated support items, or compiler
/// diagnostics when the carrier is invalid.
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
