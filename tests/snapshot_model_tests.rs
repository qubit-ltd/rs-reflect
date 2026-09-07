// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

//! Deterministic seeds retained alongside the bounded snapshot fuzzer.

#[path = "internal/registry_snapshot_model_tests.rs"]
mod registry_snapshot_model;

#[test]
fn test_snapshot_order_duplicates_and_capability_isolation() {
    for seed in [
        vec![],
        vec![0; 10],
        vec![1, 0, 0, 0, 7, 1, 0, 1, 0, 8],
        vec![0, 0, 0, 0, 0, 1, 0, 1, 0, 7, 0, 1, 2, 0, 0],
        vec![255; 8192],
    ] {
        registry_snapshot_model::check(&seed);
    }
}
