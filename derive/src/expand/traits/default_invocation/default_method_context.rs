// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Shared inputs used to emit one default trait method.

use proc_macro2::Ident;
use proc_macro2::TokenStream;

use crate::expand::ExpansionContext;

/// Inputs that remain constant while one default method is generated.
pub(super) struct DefaultMethodContext<'a> {
    /// Source trait path used by the generated adapter.
    pub(super) trait_path: &'a TokenStream,
    /// Generic parameters declared on the generated helper method.
    pub(super) helper_generic_parameters: &'a [TokenStream],
    /// Marker type identifying the reflected trait definition.
    pub(super) marker_type: &'a TokenStream,
    /// Generic arguments applied to the concrete target type.
    pub(super) target_generic_arguments: &'a TokenStream,
    /// Generated provider for the reflected trait specification.
    pub(super) spec_provider: &'a Ident,
    /// Runtime codegen module path.
    pub(super) codegen: &'a TokenStream,
    /// Runtime facade path.
    pub(super) facade: &'a TokenStream,
    /// Expansion context shared by generated method helpers.
    pub(super) context: &'a ExpansionContext,
    /// Type parameter names declared by the source trait.
    pub(super) trait_type_parameters: &'a [String],
}
