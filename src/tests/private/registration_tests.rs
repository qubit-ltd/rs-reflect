// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Tests benchmark registry preparation and production aggregation facades.

use crate::__private::registration::aggregate_benchmark_registry_facts;
use crate::__private::registration::prepare_benchmark_registry_facts;

/// Verifies prepared synthetic facts pass through production aggregation.
#[test]
fn test_benchmark_registry_facts_use_production_aggregation() {
    let facts = prepare_benchmark_registry_facts(4);
    let registry =
        aggregate_benchmark_registry_facts(&facts).expect("distinct prepared facts must aggregate successfully");
    assert!(registry.types().is_empty());
}
