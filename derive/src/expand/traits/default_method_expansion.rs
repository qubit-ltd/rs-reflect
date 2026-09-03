// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Generated pieces associated with reflected trait default methods.

use proc_macro2::TokenStream;

/// Generated pieces associated with reflected trait default methods.
pub(super) struct DefaultMethodExpansion {
    /// Generated adapter items appended to the reflected trait.
    pub(super) adapter_items: Vec<TokenStream>,
    /// Descriptor entries for generated default-method adapters.
    pub(super) adapter_entries: Vec<TokenStream>,
    /// Descriptor entries explaining unavailable default-method adapters.
    pub(super) unavailable_reason_entries: Vec<TokenStream>,
}
