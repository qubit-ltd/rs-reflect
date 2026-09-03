// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Shared token groups for reflected trait metadata and dyn applications.

use proc_macro2::TokenStream;

/// Token groups used by both trait definitions and applied descriptors.
pub(super) struct TraitMetadata {
    /// Method descriptor expressions.
    pub(super) methods: Vec<TokenStream>,
    /// Associated type descriptor expressions.
    pub(super) associated_types: Vec<TokenStream>,
    /// Associated constant descriptor expressions.
    pub(super) associated_consts: Vec<TokenStream>,
    /// Generic parameter descriptor expressions.
    pub(super) parameters: Vec<TokenStream>,
    /// Generic where-predicate descriptor expressions.
    pub(super) where_predicates: Vec<TokenStream>,
}
