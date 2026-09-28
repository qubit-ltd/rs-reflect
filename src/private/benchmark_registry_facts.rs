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
pub struct BenchmarkRegistryFacts(
    /// Precomputed adapter-free fragments used by the aggregation benchmark.
    crate::registry::BenchmarkRegistryFacts,
);

/// Prepares adapter-free capability facts outside the measured aggregation.
///
/// # Parameters
///
/// - `fragment_count`: Number of unique synthetic capability fragments.
///
/// # Returns
///
/// Returns reusable facts that can be aggregated repeatedly by a benchmark.
#[doc(hidden)]
#[must_use]
pub fn prepare_benchmark_registry_facts(fragment_count: usize) -> BenchmarkRegistryFacts {
    BenchmarkRegistryFacts(crate::registry::prepare_benchmark_registry_facts(fragment_count))
}

/// Runs production post-materialization validation, indexing, and freezing on
/// prepared benchmark facts.
///
/// # Parameters
///
/// - `facts`: Adapter-free facts prepared before the measured operation.
///
/// # Returns
///
/// Returns the immutable registry snapshot.
///
/// # Errors
///
/// Returns a validation error if the prepared facts conflict or are
/// inconsistent.
#[doc(hidden)]
pub fn aggregate_benchmark_registry_facts(facts: &BenchmarkRegistryFacts) -> Result<ReflectRegistry, RegistryError> {
    let BenchmarkRegistryFacts(facts) = facts;
    crate::registry::aggregate_benchmark_registry_facts(facts)
}
