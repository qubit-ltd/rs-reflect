// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Inputs required to construct a thread-safe default method adapter.

use proc_macro2::Ident;
use proc_macro2::TokenStream;

/// Inputs required to build an adapter for a default trait method.
pub(super) struct ThreadSafeDefaultAdapterContext<'a> {
    /// Trait path associated with the implementation.
    pub(super) trait_path: &'a TokenStream,
    /// Generic parameters available to generated adapter types.
    pub(super) helper_generic_parameters: &'a [TokenStream],
    /// Marker type identifying the reflected trait.
    pub(super) marker_type: &'a TokenStream,
    /// Generic arguments applied to the target type.
    pub(super) target_generic_arguments: &'a TokenStream,
    /// Runtime codegen module path.
    pub(super) codegen: &'a TokenStream,
    /// Generated proof that the value mode is eligible.
    pub(super) value_eligibility: &'a Ident,
}
