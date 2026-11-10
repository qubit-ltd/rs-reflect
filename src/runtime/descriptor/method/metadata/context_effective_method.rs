// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Provides the contexteffectivemethod operation used to resolve or dispatch
/// reflected methods.
#[derive(Clone)]
pub(crate) struct ContextEffectiveMethod {
    /// Returns the static effective method record shared by the catalog view.
    pub(crate) effective: EffectiveMethod,
    /// Registered concrete generic instantiations available for this method.
    pub(crate) specializations: std::sync::Arc<[ResolvedMethodSpecialization]>,
    /// Provides the implementation operation used to resolve or dispatch
    /// reflected methods.
    pub(crate) implementation:
        Option<&'static crate::runtime::descriptor::impl_descriptor::metadata::ImplApplicationMetadata>,
}
