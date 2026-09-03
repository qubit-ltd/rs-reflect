// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Criterion benchmark for hot external-supertrait cache lookup.

use criterion::Criterion;
use criterion::black_box;
use criterion::criterion_group;
use criterion::criterion_main;
use qubit_reflect::__private::codegen_v2::descriptor::external_supertrait;

/// Registers a hot lookup after priming the exact external application.
fn trait_cache_lookup(criterion: &mut Criterion) {
    let first =
        external_supertrait::<dyn std::fmt::Display>("benchmark.external.display", "std::fmt::Display", Vec::new());
    black_box(first);

    criterion.bench_function("trait_cache/hot_external_supertrait", |bench| {
        bench.iter(|| {
            black_box(external_supertrait::<dyn std::fmt::Display>(
                "benchmark.external.display",
                "std::fmt::Display",
                Vec::new(),
            ))
        });
    });
}

criterion_group!(benches, trait_cache_lookup);
criterion_main!(benches);
