// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Criterion benchmark for hot lookup of a retained external trait application.

use std::hint::black_box;

use criterion::Criterion;
use criterion::criterion_group;
use criterion::criterion_main;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::ReflectionContext;

#[Reflect]
struct ExternalDisplayTarget;

#[ReflectImpl]
impl std::fmt::Display for ExternalDisplayTarget {
    #[reflect(skip)]
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("external-display-benchmark")
    }
}

/// Registers a hot lookup after priming the exact external application path.
fn trait_cache_lookup(criterion: &mut Criterion) {
    let context = ReflectionContext::global();
    let first = context
        .trait_applications_by_path("std::fmt::Display")
        .expect("global context must build");
    assert_eq!(first.len(), 1);
    black_box(first);

    criterion.bench_function("trait_cache/hot_external_trait_application", |bench| {
        bench.iter(|| {
            black_box(
                context
                    .trait_applications_by_path("std::fmt::Display")
                    .expect("global context remains valid"),
            )
        });
    });
}

criterion_group!(benches, trait_cache_lookup);
criterion_main!(benches);
