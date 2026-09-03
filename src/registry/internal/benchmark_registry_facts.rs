// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Benchmark-only prepared registry facts.

use std::any::TypeId;

use crate::capability::CapabilityDescriptor;
use crate::capability::CapabilityKey;
use crate::error::RegistryError;
use crate::identity::CapabilityId;
use crate::identity::FragmentIdentity;
use crate::registry::fragment::CapabilityRegistration;
use crate::registry::fragment::FragmentKind;
use crate::registry::fragment::FragmentPayload;
use crate::registry::fragment::RuntimeIdentity;
use crate::registry::internal::MaterializedFragment;
use crate::registry::internal::benchmark_target::BenchmarkTarget;
use crate::registry::internal::fact_row::FactRow;
use crate::registry::registry::ReflectRegistry;

/// Prepared adapter-free capability facts used by registry aggregation.
pub(crate) struct BenchmarkRegistryFacts {
    fragments: Box<[FactRow]>,
}

impl BenchmarkRegistryFacts {
    /// Aggregates the prepared facts through the production registry path.
    pub(crate) fn aggregate(&self) -> Result<ReflectRegistry, RegistryError> {
        aggregate_prepared_facts(self)
    }
}

/// Prepares unique adapter-free capability facts outside benchmark timing.
pub(crate) fn prepare_benchmark_registry_facts(fragment_count: usize) -> BenchmarkRegistryFacts {
    let fragments = (0..fragment_count)
        .rev()
        .map(|index| {
            let capability_id = CapabilityId::new(&format!("benchmark.registry.fragment{index}"))
                .expect("generated benchmark capability ID must be valid");
            FactRow {
                identity: FragmentIdentity::new(
                    "qubit-reflect-benchmark",
                    "synthetic",
                    u32::try_from(index).expect("benchmark fragment index must fit u32"),
                    1,
                    "capability",
                    index as u64,
                ),
                target_type_id: TypeId::of::<BenchmarkTarget>(),
                descriptor: CapabilityDescriptor::without_adapter(CapabilityKey::<u8>::new(capability_id)),
            }
        })
        .collect();
    BenchmarkRegistryFacts { fragments }
}

/// Aggregates prepared facts through the production registry path.
pub(crate) fn aggregate_benchmark_registry_facts(
    facts: &BenchmarkRegistryFacts,
) -> Result<ReflectRegistry, RegistryError> {
    facts.aggregate()
}

/// Materializes prepared facts for the benchmark-facing aggregation method.
fn aggregate_prepared_facts(facts: &BenchmarkRegistryFacts) -> Result<ReflectRegistry, RegistryError> {
    let fragments = facts
        .fragments
        .iter()
        .map(|fact| MaterializedFragment {
            identity: fact.identity.clone(),
            declared_kind: FragmentKind::Capability,
            declared_target: RuntimeIdentity::Capabilities {
                target_type_id: fact.target_type_id,
            },
            payload: FragmentPayload::Capability(CapabilityRegistration::new(
                fact.target_type_id,
                vec![fact.descriptor.clone()],
            )),
        })
        .collect();
    super::super::registry_builder::validate_and_freeze_materialized(fragments)
}
