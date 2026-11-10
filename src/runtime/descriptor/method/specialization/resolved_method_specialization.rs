// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// A compile-time-bound implementation of one generic method.
#[derive(Clone)]
pub(crate) struct ResolvedMethodSpecialization {
    /// Generic arguments supplied when selecting a method specialization.
    pub(crate) arguments: ResolvedMethodArguments,
    /// Metadata describing each generic parameter accepted by the method.
    pub(crate) argument_metadata: Option<std::sync::Arc<[GenericArgumentMetadata]>>,
    /// Invocation availability after generic arguments are checked.
    pub(crate) availability: InvocationAvailability,
    /// Generated adapters for supported value modes and invocation variants.
    pub(crate) adapters: InvocationAdapters,
    /// Concrete call parameters supplied by a generated method binding.
    pub(crate) parameters: Option<&'static [ParameterMetadata]>,
}
