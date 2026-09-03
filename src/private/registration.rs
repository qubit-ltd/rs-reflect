// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow type-file-name
//! Hidden distributed-registration protocol for generated macro output.

use std::any::TypeId;
use std::sync::OnceLock;

use crate::capability::CapabilityKey;
use crate::error::RegistryError;
use crate::registry::ReflectRegistry;
#[doc(hidden)]
pub use crate::registry::fragment::CapabilityRegistration;
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

/// Returns whether the unified inventory declares one executable typed
/// capability for an exact concrete type.
///
/// This declaration probe does not validate conflicts; registry construction
/// remains the sole conflict authority.
#[doc(hidden)]
pub fn has_registered_capability<T: 'static, A: 'static>(key: CapabilityKey<A>) -> bool {
    inventory::iter::<RegistrationFragment>
        .into_iter()
        .filter(|fragment| fragment.kind() == FragmentKind::Capability)
        .any(|fragment| match fragment.build() {
            FragmentPayload::Capability(registration) if registration.target_type_id() == TypeId::of::<T>() => {
                registration.descriptors().iter().any(|descriptor| {
                    descriptor.id() == key.id()
                        && descriptor.adapter_type() == key.adapter_type()
                        && descriptor.has_adapter()
                })
            }
            _ => false,
        })
}

/// Builds and validates an isolated registry snapshot from static fragments.
///
/// This entry point exists for generated-code integration and conformance
/// fixtures. Ordinary applications should call [`ReflectRegistry::initialize`].
/// Returns [`RegistryError`] only after checking the complete fragment set.
#[doc(hidden)]
pub fn build_registry(fragments: &[&'static RegistrationFragment]) -> Result<ReflectRegistry, RegistryError> {
    crate::registry::build_registry(fragments)
}

/// Prepared synthetic facts for post-materialization registry aggregation
/// benchmarks.
///
/// This type is absent unless the non-default `bench-internals` feature is
/// enabled.
#[cfg(feature = "bench-internals")]
#[doc(hidden)]
pub struct BenchmarkRegistryFacts(crate::registry::BenchmarkRegistryFacts);

/// Prepares adapter-free capability facts outside the measured aggregation.
#[cfg(feature = "bench-internals")]
#[doc(hidden)]
#[must_use]
pub fn prepare_benchmark_registry_facts(fragment_count: usize) -> BenchmarkRegistryFacts {
    BenchmarkRegistryFacts(crate::registry::prepare_benchmark_registry_facts(fragment_count))
}

/// Runs production post-materialization validation, indexing, and freezing on
/// prepared benchmark facts.
#[cfg(feature = "bench-internals")]
#[doc(hidden)]
pub fn aggregate_benchmark_registry_facts(facts: &BenchmarkRegistryFacts) -> Result<ReflectRegistry, RegistryError> {
    let BenchmarkRegistryFacts(facts) = facts;
    crate::registry::aggregate_benchmark_registry_facts(facts)
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
