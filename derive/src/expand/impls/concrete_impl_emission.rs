// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Token groups needed to emit one concrete reflected implementation.

use proc_macro2::Ident;
use proc_macro2::TokenStream;

/// Fully materialized token groups needed to emit one concrete impl module.
pub(super) struct ConcreteImplEmission {
    /// Original impl tokens retained in generated output.
    pub(super) retained: TokenStream,
    /// Generated module identifier receiving the registration fragment.
    pub(super) module: Ident,
    /// Compile-time witness for a selected generic specialization.
    pub(super) applicability_witness: Option<TokenStream>,
    /// Generated invocation adapter functions.
    pub(super) invocation_adapter_definitions: Vec<TokenStream>,
    /// Generated adapters for explicitly specialized methods.
    pub(super) generic_specialization_adapter_definitions: Vec<TokenStream>,
    /// Runtime facade path used by generated code.
    pub(super) facade: TokenStream,
    /// Source declaration line retained in fragment identity.
    pub(super) line: u32,
    /// Source declaration column retained in fragment identity.
    pub(super) column: u32,
    /// Stable fingerprint retained in fragment identity.
    pub(super) fingerprint: u64,
    /// Concrete impl target type.
    pub(super) target: TokenStream,
    /// Generated trait descriptor setup.
    pub(super) trait_setup: TokenStream,
    /// Generated impl-definition setup.
    pub(super) definition_setup: TokenStream,
    /// Generated method descriptor entries.
    pub(super) method_entries: Vec<TokenStream>,
    /// Generated invocation adapter descriptor entries.
    pub(super) invocation_adapter_entries: Vec<TokenStream>,
    /// Reasons emitted for unavailable invocation adapters.
    pub(super) invocation_unavailable_reason_entries: Vec<TokenStream>,
    /// Generated method specialization descriptor entries.
    pub(super) method_specialization_entries: Vec<TokenStream>,
    /// Generated implemented-trait descriptor expression.
    pub(super) implemented_trait: TokenStream,
    /// Generated associated-type binding match arms.
    pub(super) associated_type_binding_arms: Vec<TokenStream>,
    /// Generated specialized associated-type resolver match arms.
    pub(super) specialized_associated_type_resolver_arms: Vec<TokenStream>,
    /// Generated associated-constant implementation match arms.
    pub(super) associated_const_override_arms: Vec<TokenStream>,
    /// Generated associated-constant reader match arms.
    pub(super) associated_const_reader_override_arms: Vec<TokenStream>,
    /// Generated concrete impl generic arguments.
    pub(super) impl_arguments: TokenStream,
    /// Generated external-trait registration fragment.
    pub(super) external_registration: TokenStream,
}
