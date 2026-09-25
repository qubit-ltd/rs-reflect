// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Prepared synthetic facts for post-materialization registry aggregation
//! benchmarks.

use crate::error::RegistryError;
use crate::registry::ReflectRegistry;

/// Prepared synthetic facts for post-materialization registry aggregation
/// benchmarks.
#[doc(hidden)]
pub struct BenchmarkRegistryFacts(crate::registry::BenchmarkRegistryFacts);

/// Prepares adapter-free capability facts outside the measured aggregation.
#[doc(hidden)]
#[must_use]
pub fn prepare_benchmark_registry_facts(fragment_count: usize) -> BenchmarkRegistryFacts {
    BenchmarkRegistryFacts(crate::registry::prepare_benchmark_registry_facts(fragment_count))
}

/// Runs production post-materialization validation, indexing, and freezing on
/// prepared benchmark facts.
#[doc(hidden)]
pub fn aggregate_benchmark_registry_facts(facts: &BenchmarkRegistryFacts) -> Result<ReflectRegistry, RegistryError> {
    let BenchmarkRegistryFacts(facts) = facts;
    crate::registry::aggregate_benchmark_registry_facts(facts)
}
