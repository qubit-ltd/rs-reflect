// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// The callable implementation selected for a target type.
#[derive(Clone, Copy)]
pub struct EffectiveMethod {
    /// Concrete target type whose method metadata is being registered.
    pub(crate) target: fn() -> &'static crate::runtime::descriptor::metadata::TypeMetadata,
    /// Static declaration facts shared by descriptors created in a reflection
    /// context.
    pub(crate) metadata: &'static MethodMetadata,
    /// Source method declaration used to build the effective method metadata.
    pub(crate) source: MethodImplementationSource,
    /// Invocation availability after generic arguments are checked.
    pub(crate) availability: MethodAvailability,
    /// Registered concrete generic instantiations available for this method.
    pub(crate) specializations: &'static [ResolvedMethodSpecialization],
    /// Static specifications used to resolve generic arguments at invocation
    /// time.
    pub(crate) specialization_specs: &'static [MethodSpecializationSpec],
    /// Generated adapters for supported value modes and invocation variants.
    pub(crate) adapters: Option<InvocationAdapters>,
    /// Concrete parameter metadata retained only for generated method bindings.
    pub(crate) specialization_parameters: Option<&'static [ParameterMetadata]>,
}
