// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::method_namespace_key::MethodNamespaceKey;
use crate::runtime::descriptor::impl_descriptor::ImplApplicationMetadata;
use crate::runtime::descriptor::method::EffectiveMethod;
use crate::runtime::descriptor::method::ResolvedMethodSpecialization;

/// Collected effective method and validation data for one target namespace.
pub(super) struct MethodEntry {
    /// Namespace used to identify the method declaration.
    pub(super) namespace: MethodNamespaceKey,
    /// Effective method metadata and invocation adapters.
    pub(super) method: EffectiveMethod,
    /// Source implementation application, when the method came from an impl.
    pub(super) implementation: Option<&'static ImplApplicationMetadata>,
    /// Concrete generic specializations resolved for the method.
    pub(super) specializations: Vec<ResolvedMethodSpecialization>,
    /// Indicates whether specialization metadata still awaits its declaration.
    pub(super) pending_specs: bool,
}
