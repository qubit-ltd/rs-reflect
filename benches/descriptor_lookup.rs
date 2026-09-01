// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Baseline executable for descriptor lookup benchmarking.

use std::hint::black_box;
use std::time::Instant;

use qubit_reflect::TypeDescriptor;

/// Measures repeated lookup of representative built-in descriptors.
fn main() {
    let cold_started = Instant::now();
    black_box(TypeDescriptor::of::<Vec<Option<String>>>());
    println!("cold descriptor initialization: {:?}", cold_started.elapsed());

    let hot_started = Instant::now();
    for _ in 0..100_000 {
        black_box(TypeDescriptor::of::<u64>());
        black_box(TypeDescriptor::of::<Vec<String>>());
    }
    println!("hot descriptor lookup: {:?}", hot_started.elapsed());
}
