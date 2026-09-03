// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Post-materialization registry aggregation and frozen-lookup baselines.
//!
//! The aggregation cases validate, index, and freeze fresh registry snapshots
//! containing exactly 1, 100, and 10,000 prepared adapter-free capability
//! fragments. Fragment string construction and ID parsing happen before the
//! timer. The production process-global registry is measured separately
//! because its `OnceLock` can only initialize once per process.

use std::any::TypeId;

use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::black_box;
use criterion::criterion_group;
use criterion::criterion_main;
use qubit_reflect as reflect;
use reflect::__private::codegen_v2::registration::FragmentKind;
use reflect::__private::codegen_v2::registration::FragmentPayload;
use reflect::__private::codegen_v2::registration::RegistrationFragment;
use reflect::__private::codegen_v2::registration::RuntimeIdentity;
use reflect::__private::codegen_v2::registration::StaticFragmentIdentity;
use reflect::__private::testing::aggregate_benchmark_registry_facts;
use reflect::__private::testing::prepare_benchmark_registry_facts;
use reflect::TypeDescriptor;
use reflect::registry::ReflectRegistry;

struct RegistryBenchmarkType;

static REGISTRY_BENCHMARK_DESCRIPTOR: TypeDescriptor =
    reflect::__private::codegen_v2::descriptor::opaque_root::<RegistryBenchmarkType>("registry-benchmark");

/// Returns the runtime identity used by the benchmark fixture.
fn benchmark_runtime_identity() -> RuntimeIdentity {
    RuntimeIdentity::Type(TypeId::of::<RegistryBenchmarkType>())
}

/// Returns the descriptor payload used by the benchmark fixture.
fn benchmark_payload() -> FragmentPayload {
    FragmentPayload::Type(&REGISTRY_BENCHMARK_DESCRIPTOR)
}

static REGISTRY_BENCHMARK_FRAGMENT: RegistrationFragment = RegistrationFragment::new(
    FragmentKind::Type,
    StaticFragmentIdentity::new("registry-bench", "single", 1, 1, "type", 1),
    benchmark_runtime_identity,
    benchmark_payload,
);

reflect::__private::codegen_v2::inventory::submit! {
    REGISTRY_BENCHMARK_FRAGMENT
}

/// Registers aggregation, global initialization, and frozen lookup cases.
fn registry_operations(criterion: &mut Criterion) {
    let one_fragment = prepare_benchmark_registry_facts(1);
    let one_hundred_fragments = prepare_benchmark_registry_facts(100);
    let ten_thousand_fragments = prepare_benchmark_registry_facts(10_000);
    aggregate_benchmark_registry_facts(&one_fragment).expect("one-fragment aggregation setup must succeed");
    aggregate_benchmark_registry_facts(&one_hundred_fragments).expect("100-fragment aggregation setup must succeed");
    aggregate_benchmark_registry_facts(&ten_thousand_fragments)
        .expect("10,000-fragment aggregation setup must succeed");

    let mut aggregation = criterion.benchmark_group("registry/aggregation");
    aggregation.bench_with_input(BenchmarkId::from_parameter(1), &one_fragment, |bench, facts| {
        bench.iter(|| black_box(aggregate_benchmark_registry_facts(facts)));
    });
    aggregation.bench_with_input(
        BenchmarkId::from_parameter(100),
        &one_hundred_fragments,
        |bench, facts| bench.iter(|| black_box(aggregate_benchmark_registry_facts(facts))),
    );
    aggregation.sample_size(10).bench_with_input(
        BenchmarkId::from_parameter(10_000),
        &ten_thousand_fragments,
        |bench, facts| bench.iter(|| black_box(aggregate_benchmark_registry_facts(facts))),
    );
    aggregation.finish();

    let registry = ReflectRegistry::initialize().expect("production registry initialization");
    criterion.bench_function("registry/hot_global_initialize", |bench| {
        bench.iter(|| black_box(ReflectRegistry::initialize()));
    });

    let mut lookup = criterion.benchmark_group("registry/frozen_lookup_batch");
    for batch_size in [1_usize, 100, 10_000] {
        lookup.bench_with_input(BenchmarkId::from_parameter(batch_size), &batch_size, |bench, size| {
            bench.iter(|| {
                for _ in 0..*size {
                    black_box((
                        registry.get(TypeId::of::<RegistryBenchmarkType>()),
                        registry.find_by_query_name("registry-benchmark").len(),
                    ));
                }
            });
        });
    }
    lookup.finish();
}

criterion_group!(benches, registry_operations);
criterion_main!(benches);
