// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Result type for compile-time-selected thread-safe adapter generation.

use proc_macro2::TokenStream;

/// One compile-time-selected thread-safe adapter provider.
pub(in crate::expand::impls) struct ThreadSafeDefinition {
    /// Generated adapter implementation tokens.
    pub(in crate::expand::impls) tokens: TokenStream,
    /// Generated selector used to choose the adapter when bounds are met.
    pub(in crate::expand::impls) selector: TokenStream,
}
