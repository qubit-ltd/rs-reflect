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
use std::time::Instant;

use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::criterion_group;
use criterion::criterion_main;
use qubit_reflect::__private::codegen_v4::registration::benchmark_fragment_graph;
use qubit_reflect::__private::codegen_v4::registration::benchmark_select_fragments;
use qubit_reflect::__private::codegen_v4::registration::benchmark_select_linked_inventory;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectionContext;

#[Reflect]
struct RegistryBenchmarkType {
    pub value: u64,
}

/// Registers global context initialization and immutable lookup cases.
fn registry_operations(criterion: &mut Criterion) {
    let context = ReflectionContext::global();
    context
        .initialize()
        .expect("production context initialization");

    criterion.bench_function("context/hot_global_initialize", |bench| {
        bench.iter(|| black_box(context.initialize()));
    });

    let cold_start = Instant::now();
    black_box(
        ReflectionContext::builder()
            .include_linked_for::<RegistryBenchmarkType>()
            .build()
            .expect("first explicit linked context"),
    );
    eprintln!(
        "first_explicit_linked_build_ns={}",
        cold_start.elapsed().as_nanos()
    );

    criterion.bench_function("context/repeated_explicit_linked_build", |bench| {
        bench.iter(|| {
            black_box(
                ReflectionContext::builder()
                    .include_linked_for::<RegistryBenchmarkType>()
                    .build()
                    .expect("linked benchmark context"),
            )
        });
    });
    criterion.bench_function("selection/actual_inventory_phase", |bench| {
        bench.iter(|| black_box(benchmark_select_linked_inventory::<RegistryBenchmarkType>()));
    });

    let mut selection = criterion.benchmark_group("selection/synthetic_slice");
    for count in [32_usize, 128, 512] {
        for depth in [1_usize, 8, 32] {
            for shape in [
                "chain",
                "reverse_chain",
                "branch",
                "cycle",
                "shared",
                "unrelated",
            ] {
                let fragments = benchmark_fragment_graph(count, depth, shape);
                selection.bench_function(
                    format!("{shape}/fragments_{count}/depth_{depth}"),
                    |bench| {
                        bench.iter(|| {
                            black_box(benchmark_select_fragments(black_box(&fragments), 0))
                        })
                    },
                );
            }
        }
    }
    selection.finish();

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
        lookup.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            &batch_size,
            |bench, size| {
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
            },
        );
    }
    lookup.finish();
}

criterion_group!(benches, registry_operations);
criterion_main!(benches);
