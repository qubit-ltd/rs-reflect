// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Criterion benchmarks for descriptor lookup.

use criterion::Criterion;
use criterion::black_box;
use criterion::criterion_group;
use criterion::criterion_main;
use qubit_reflect::TypeDescriptor;

/// Registers cold-shape and hot-interner descriptor cases.
fn descriptor_lookup(criterion: &mut Criterion) {
    criterion.bench_function("descriptor/cold_nested_shape", |bench| {
        bench.iter(|| black_box(TypeDescriptor::of::<Vec<Option<String>>>()));
    });
    criterion.bench_function("descriptor/hot_builtin_pair", |bench| {
        bench.iter(|| {
            black_box(TypeDescriptor::of::<u64>());
            black_box(TypeDescriptor::of::<Vec<String>>());
        });
    });
}

criterion_group!(benches, descriptor_lookup);
criterion_main!(benches);
