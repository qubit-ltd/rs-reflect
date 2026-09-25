// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow type-file-name
//! Hidden distributed-registration protocol for generated macro output.

use std::sync::OnceLock;

use crate::error::RegistryError;
use crate::registry::ReflectRegistry;
#[doc(hidden)]
pub use crate::registry::fragment::CapabilityRegistration;
#[doc(hidden)]
pub use crate::registry::fragment::CapabilityTarget;
#[doc(hidden)]
pub use crate::registry::fragment::FragmentKind;
#[doc(hidden)]
pub use crate::registry::fragment::FragmentPayload;
#[doc(hidden)]
pub use crate::registry::fragment::RegistrationFragment;
#[doc(hidden)]
pub use crate::registry::fragment::RuntimeIdentity;
#[doc(hidden)]
pub use crate::registry::fragment::StaticFragmentIdentity;

/// Builds and validates an isolated registry snapshot from static fragments.
///
/// This entry point exists for generated-code integration and conformance
/// fixtures. Ordinary applications should call [`ReflectRegistry::initialize`].
/// Returns [`RegistryError`] only after checking the complete fragment set.
#[doc(hidden)]
pub fn build_registry(fragments: &[&'static RegistrationFragment]) -> Result<ReflectRegistry, RegistryError> {
    crate::registry::build_registry(fragments)
}

#[cfg(feature = "bench-internals")]
pub use super::benchmark_registry_facts::aggregate_benchmark_registry_facts;
#[cfg(feature = "bench-internals")]
pub use super::benchmark_registry_facts::prepare_benchmark_registry_facts;

/// Builds an effective method view from prepared impl facts for benchmarks.
#[cfg(feature = "bench-internals")]
#[doc(hidden)]
pub fn build_benchmark_effective_type_view(
    implementations: &[&'static crate::descriptor::ImplDescriptor],
) -> crate::registry::EffectiveTypeView {
    crate::registry::build_benchmark_effective_type_view(implementations)
}

/// Initializes a caller-supplied cache from static fragments.
///
/// The first complete success or failure is retained in `cache`; later calls
/// return the same registry reference or a clone of the cached error even when
/// supplied a different fragment slice.
#[doc(hidden)]
pub fn initialize_registry(
    cache: &'static OnceLock<Result<ReflectRegistry, RegistryError>>,
    fragments: &'static [&'static RegistrationFragment],
) -> Result<&'static ReflectRegistry, RegistryError> {
    crate::registry::initialize_registry(cache, fragments)
}
