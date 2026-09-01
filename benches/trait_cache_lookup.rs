// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Baseline executable for hot external-supertrait cache lookup benchmarking.

use std::hint::black_box;

use qubit_reflect::__private::external_supertrait;

/// Measures repeated lookup after the exact external application is cached.
fn main() {
    let first = external_supertrait::<dyn std::fmt::Display>(
        "benchmark.external.display",
        "std::fmt::Display",
        Vec::new(),
    );
    black_box(first);

    for _ in 0..100_000 {
        black_box(external_supertrait::<dyn std::fmt::Display>(
            "benchmark.external.display",
            "std::fmt::Display",
            Vec::new(),
        ));
    }
}
