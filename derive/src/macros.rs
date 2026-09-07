// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::entry;
use crate::ir::MacroKind;
use proc_macro::TokenStream;

/// Derives structural reflection for a struct or enum.
///
/// Generic facade macros may request `definition_provider_v2 = identifier`.
/// It emits the caller-named provider using the facade's codegen protocol.
/// Type-level helpers include `rename`, `opaque`, `capabilities(...)`, and
/// `crate = path`; fields and variants support their corresponding policies.
/// The macro rejects unions, duplicate query names, invalid helper placements,
/// and conflicting policies with source-oriented compiler diagnostics.
///
/// See the [user guide](https://github.com/qubit-ltd/rs-reflect/blob/main/doc/2026-08-29-qubit-reflect-user-guide.md)
/// for construction, capability, generic, and facade examples.
pub fn derive_reflect(input: TokenStream) -> TokenStream {
    entry::process_macro(MacroKind::Derive, TokenStream::new(), input)
}

/// Reflects a trait declaration and its metadata contract.
///
/// The macro records methods, associated types, associated constants, generic
/// parameters, and supported supertraits. Dynamic invocation is emitted only
/// when the method signature can cross the reflection boundary safely;
/// unsupported methods retain a structured unavailable reason.
pub fn reflect(attribute: TokenStream, item: TokenStream) -> TokenStream {
    entry::process_macro(MacroKind::Trait, attribute, item)
}

/// Reflects an inherent or trait implementation.
///
/// `specialize(...)` registers selected concrete instances of a generic impl;
/// `external_trait_id` gives a stable identity to an external trait
/// implementation; and `crate = path` selects a downstream runtime facade.
/// `thread_safe` and `catch_unwind` validate the generated boundary before
/// emitting invocation code.
pub fn reflect_impl(attribute: TokenStream, item: TokenStream) -> TokenStream {
    entry::process_macro(MacroKind::Impl, attribute, item)
}
