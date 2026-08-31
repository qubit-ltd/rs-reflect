// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Baseline executable for descriptor lookup benchmarking.

use std::hint::black_box;

use qubit_reflect::TypeDescriptor;

/// Measures repeated lookup of representative built-in descriptors.
fn main() {
    for _ in 0..100_000 {
        black_box(TypeDescriptor::of::<u64>());
        black_box(TypeDescriptor::of::<Vec<String>>());
    }
}
