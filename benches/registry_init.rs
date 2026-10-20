// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Global context initialization and frozen public lookup baselines.

use std::any::TypeId;
use std::hint::black_box;

use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::criterion_group;
use criterion::criterion_main;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectionContext;

#[Reflect]
struct RegistryBenchmarkType {
    pub value: u64,
}

/// Registers global context initialization and immutable lookup cases.
fn registry_operations(criterion: &mut Criterion) {
    let context = ReflectionContext::global();
    context.initialize().expect("production context initialization");

    criterion.bench_function("context/hot_global_initialize", |bench| {
        bench.iter(|| black_box(context.initialize()));
    });

    let target_id = TypeId::of::<RegistryBenchmarkType>();
    let target_name = "RegistryBenchmarkType";
    assert_eq!(
        context
            .type_by_id(target_id)
            .expect("benchmark type must be registered")
            .short_name(),
        target_name,
    );
    assert_eq!(RegistryBenchmarkType { value: 1 }.value, 1);

    let mut lookup = criterion.benchmark_group("context/frozen_type_lookup_batch");
    for batch_size in [1_usize, 100, 10_000] {
        lookup.bench_with_input(BenchmarkId::from_parameter(batch_size), &batch_size, |bench, size| {
            bench.iter(|| {
                for _ in 0..*size {
                    black_box((
                        context
                            .type_by_id(target_id)
                            .expect("benchmark type remains registered"),
                        context
                            .type_by_short_name(target_name)
                            .expect("benchmark type lookup remains unique"),
                    ));
                }
            });
        });
    }
    lookup.finish();
}

criterion_group!(benches, registry_operations);
criterion_main!(benches);
