// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Procedural macros for `qubit-reflect`.
//!
//! Generated code uses the runtime selected by `#[reflect(crate = path)]` and
//! reaches implementation details only through that runtime's versioned
//! `__private::codegen_v1` protocol. Applications should normally use the
//! re-exported macros from `qubit-reflect` instead of depending on this crate
//! directly.

#![forbid(unsafe_code)]

mod entry;
mod expand;
mod internal;
mod ir;
mod parse;
mod validate;

use proc_macro::TokenStream;

use ir::MacroKind;

/// Derives structural reflection for a struct or enum.
///
/// Type-level helpers include `rename`, `opaque`, `capabilities(...)`, and
/// `crate = path`. Fields and variants support `rename`, `skip`, `opaque`,
/// `read_only`, `no_construct`, and `default` where applicable. The macro
/// rejects unions, duplicate query names, invalid helper placements, and
/// conflicting policies with source-oriented compiler diagnostics.
///
/// `crate = path` is intended for downstream facades. The selected facade must
/// expose the matching `__private::codegen_v1` protocol; generated code does
/// not require it to re-export the runtime's public modules.
///
/// See the
/// [user guide](https://github.com/qubit-ltd/rs-reflect/blob/main/doc/2026-08-29-qubit-reflect-user-guide.md)
/// for construction, capability, generic, and facade examples.
#[proc_macro_derive(Reflect, attributes(reflect))]
pub fn derive_reflect(input: TokenStream) -> TokenStream {
    entry::process_macro(MacroKind::Derive, TokenStream::new(), input)
}

/// Reflects a trait declaration and its metadata contract.
///
/// The macro records methods, associated types, associated constants,
/// generics, and supported supertraits. Trait helpers include `rename`,
/// `supertrait(...)`, `external_trait(path, id = "...")`, `dyn_compatible`,
/// and `crate = path`. Method helpers include `rename`, `skip`, `no_invoke`,
/// `specialize(...)`, `thread_safe`, and `catch_unwind`.
///
/// Dynamic invocation is emitted only when the receiver, ABI, parameters, and
/// return type can cross the reflection boundary safely. Unsupported methods
/// remain described with a structured unavailable reason.
#[proc_macro_attribute]
pub fn reflect(attribute: TokenStream, item: TokenStream) -> TokenStream {
    entry::process_macro(MacroKind::Trait, attribute, item)
}

/// Reflects an inherent or trait implementation.
///
/// `specialize(...)` registers selected concrete instances of a generic impl;
/// `external_trait_id = "..."` gives a stable identity to an implementation of
/// a trait that cannot itself carry `#[reflect]`; and `crate = path` selects a
/// downstream runtime facade. Individual methods accept the invocation policy
/// helpers documented on [`reflect`].
///
/// Pre-execution validation preserves owned receiver and argument values on
/// failure. `thread_safe` requires the generated boundary to satisfy Rust's
/// `Send + Sync` constraints, while `catch_unwind` is rejected for async
/// methods and for signatures that are not unwind-safe.
#[proc_macro_attribute]
pub fn reflect_impl(attribute: TokenStream, item: TokenStream) -> TokenStream {
    entry::process_macro(MacroKind::Impl, attribute, item)
}
