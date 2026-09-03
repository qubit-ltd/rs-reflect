// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Generated generic syntax and associated-type identities for dyn roots.

use proc_macro2::TokenStream;

/// Generated generic syntax and associated-type identities for one dyn root.
pub(super) struct DynTraitGenerics {
    /// Generic parameters used by the generated impl declaration.
    pub(super) impl_declaration: TokenStream,
    /// Generic arguments used by the generated trait application.
    pub(super) trait_application: TokenStream,
    /// Arguments passed to generated descriptor factories.
    pub(super) factory_arguments: Vec<TokenStream>,
    /// Bounds required by the generated dyn root.
    pub(super) where_clause: TokenStream,
    /// Descriptor expressions for associated type bindings.
    pub(super) associated_type_arguments: Vec<TokenStream>,
}
