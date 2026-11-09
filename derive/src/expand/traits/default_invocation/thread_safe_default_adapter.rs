// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Intermediate output and inputs for thread-safe default method adapters.

use proc_macro2::Ident;
use proc_macro2::TokenStream;

/// Generated artifacts for one supported thread-safe default method.
pub(super) struct ThreadSafeDefaultAdapter {
    /// Adapter definition and its trait-bound selection implementation.
    pub(super) definition: TokenStream,
    /// Concrete adapter type name referenced by generated metadata.
    pub(super) adapter_type: Ident,
    /// Generated method that invokes the default implementation.
    pub(super) invoke: Ident,
    /// Trait bounds required to construct the adapter.
    pub(super) bounds: Vec<TokenStream>,
}
