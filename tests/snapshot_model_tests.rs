// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Deterministic seeds retained alongside the bounded context-model fuzzer.

mod internal;

#[test]
fn test_context_registration_order_and_isolation() {
    for seed in [
        vec![],
        vec![0; 10],
        vec![1, 0, 0, 0, 7, 1, 0, 1, 0, 8],
        vec![0, 0, 0, 0, 0, 1, 0, 1, 0, 7, 0, 1, 2, 0, 0],
        vec![255; 8192],
    ] {
        internal::registry_snapshot_model_tests::check(&seed);
    }
}
