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
use std::hint::black_box;
use std::time::Instant;

use qubit_reflect as reflect;
use reflect::__private::registration::FragmentKind;
use reflect::__private::registration::FragmentPayload;
use reflect::__private::registration::RegistrationFragment;
use reflect::__private::registration::RuntimeIdentity;
use reflect::__private::registration::StaticFragmentIdentity;
use reflect::__private::registration::aggregate_benchmark_registry_facts;
use reflect::__private::registration::prepare_benchmark_registry_facts;
use reflect::TypeDescriptor;
use reflect::registry::ReflectRegistry;

struct RegistryBenchmarkType;

static REGISTRY_BENCHMARK_DESCRIPTOR: TypeDescriptor =
    reflect::__private::descriptor::opaque_root::<RegistryBenchmarkType>("registry-benchmark");

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

reflect::__private::inventory::submit! {
    REGISTRY_BENCHMARK_FRAGMENT
}

/// Measures repeated execution of one benchmark operation.
fn measure_repeated<T>(name: &str, repetitions: usize, mut operation: impl FnMut() -> T) {
    let started = Instant::now();
    for _ in 0..repetitions {
        black_box(operation());
    }
    let elapsed = started.elapsed();
    println!(
        "{name}: {elapsed:?} total, {:?} per iteration",
        elapsed / repetitions as u32,
    );
}

fn main() {
    let one_fragment = prepare_benchmark_registry_facts(1);
    let one_hundred_fragments = prepare_benchmark_registry_facts(100);
    let ten_thousand_fragments = prepare_benchmark_registry_facts(10_000);
    aggregate_benchmark_registry_facts(&one_fragment).expect("one-fragment aggregation setup must succeed");
    aggregate_benchmark_registry_facts(&one_hundred_fragments).expect("100-fragment aggregation setup must succeed");
    aggregate_benchmark_registry_facts(&ten_thousand_fragments)
        .expect("10,000-fragment aggregation setup must succeed");

    measure_repeated("post-materialization aggregation: 1 fragment", 1_000, || {
        aggregate_benchmark_registry_facts(&one_fragment)
    });
    measure_repeated("post-materialization aggregation: 100 fragments", 100, || {
        aggregate_benchmark_registry_facts(&one_hundred_fragments)
    });
    measure_repeated("post-materialization aggregation: 10000 fragments", 3, || {
        aggregate_benchmark_registry_facts(&ten_thousand_fragments)
    });

    let registry = ReflectRegistry::initialize().expect("production registry initialization");
    measure_repeated("hot production registry initialize", 10_000, || {
        ReflectRegistry::initialize()
    });

    for batch_size in [1_usize, 100, 10_000] {
        measure_repeated(&format!("frozen lookup batch {batch_size}"), batch_size, || {
            (
                registry.get(TypeId::of::<RegistryBenchmarkType>()),
                registry.find_by_query_name("registry-benchmark").len(),
            )
        });
    }
}
