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

use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::criterion_group;
use criterion::criterion_main;
use qubit_reflect as reflect;
use reflect::__private::codegen_v3::registration::FragmentKind;
use reflect::__private::codegen_v3::registration::FragmentPayload;
use reflect::__private::codegen_v3::registration::RegistrationFragment;
use reflect::__private::codegen_v3::registration::RuntimeIdentity;
use reflect::__private::codegen_v3::registration::StaticFragmentIdentity;
use reflect::__private::testing::aggregate_benchmark_registry_facts;
use reflect::__private::testing::build_benchmark_effective_type_view;
use reflect::__private::testing::prepare_benchmark_registry_facts;
use reflect::TypeDescriptor;
use reflect::descriptor::ImplDefinitionDescriptor;
use reflect::descriptor::ImplDescriptor;
use reflect::descriptor::ImplKind;
use reflect::descriptor::InvocationUnavailableReason;
use reflect::descriptor::MethodDeclarationOwner;
use reflect::descriptor::MethodDescriptor;
use reflect::descriptor::MethodImplementationSource;
use reflect::descriptor::MethodInstanceDescriptor;
use reflect::descriptor::TypeDefinitionDescriptor;
use reflect::descriptor::TypeDefinitionId;
use reflect::expression::GenericDefinitionDescriptor;
use reflect::expression::TypeExpression;
use reflect::identity::FragmentIdentity;
use reflect::identity::CapabilityId;
use reflect::identity::MemberId;
use reflect::registry::ReflectRegistry;
use reflect::registry::RegistrySnapshotBuilder;
use reflect::capability::CapabilityDescriptor;
use reflect::capability::CapabilityKey;

struct RegistryBenchmarkType;

static REGISTRY_BENCHMARK_DESCRIPTOR: TypeDescriptor =
    reflect::__private::codegen_v3::descriptor::opaque_root::<RegistryBenchmarkType>("registry-benchmark");

/// Resolves the benchmark target used by synthetic method implementations.
fn benchmark_target_type() -> &'static TypeDescriptor {
    &REGISTRY_BENCHMARK_DESCRIPTOR
}

/// Creates distinct inherent method implementations for a synthetic type.
fn prepare_effective_method_facts(method_count: usize) -> Vec<&'static ImplDescriptor> {
    let generic_definition = Box::leak(Box::new(GenericDefinitionDescriptor::new([], [])));
    (0..method_count)
        .map(|index| {
            let impl_identity = FragmentIdentity::new(
                "registry-bench",
                "effective_view",
                index as u32 + 1,
                1,
                "impl",
                index as u64 + 1,
            );
            let definition = Box::leak(Box::new(
                ImplDefinitionDescriptor::new(
                    impl_identity.clone(),
                    TypeExpression::Parameter("Self".into()),
                    ImplKind::Inherent,
                    None,
                    generic_definition,
                )
                .expect("synthetic inherent impl definition must build"),
            ));
            let method_identity = FragmentIdentity::new(
                "registry-bench",
                "effective_view",
                index as u32 + 1,
                2,
                "method",
                index as u64 + 1,
            );
            let method_name = Box::leak(format!("method_{index}").into_boxed_str());
            let method = Box::leak(Box::new(
                MethodDescriptor::builder(
                    MemberId::new("registry-bench::effective_view", "method", index, method_identity),
                    method_name,
                    method_name,
                    MethodDeclarationOwner::Impl(definition),
                )
                .build(),
            ));
            let instance = MethodInstanceDescriptor::with_arguments(
                method,
                None,
                MethodImplementationSource::Declared,
                None,
                Box::new([]),
                Box::new([InvocationUnavailableReason::DisabledByPolicy]),
            )
            .expect("synthetic method instance must build");
            Box::leak(Box::new(
                ImplDescriptor::builder(definition, benchmark_target_type)
                    .methods(std::slice::from_ref(method))
                    .method_instances(vec![instance])
                    .build()
                    .expect("synthetic inherent impl must build"),
            )) as &'static ImplDescriptor
        })
        .collect()
}

/// Returns the runtime identity used by the benchmark fixture.
fn benchmark_runtime_identity() -> RuntimeIdentity {
    RuntimeIdentity::Type(TypeId::of::<RegistryBenchmarkType>())
}

/// Returns the descriptor payload used by the benchmark fixture.
fn benchmark_payload() -> FragmentPayload {
    FragmentPayload::Type(&REGISTRY_BENCHMARK_DESCRIPTOR)
}

/// Builds an isolated registry with the requested number of source entries.
fn prepare_source_lookup_registry(capability_count: usize) -> ReflectRegistry {
    let mut builder = RegistrySnapshotBuilder::new();
    let capabilities = (0..capability_count)
        .map(|index| {
            let id: &'static str = Box::leak(format!("registry.bench_source_{index}").into_boxed_str());
            CapabilityDescriptor::without_adapter(CapabilityKey::<()>::new(
                CapabilityId::new(id).expect("benchmark capability ID is valid"),
            ))
        })
        .collect();
    let source = FragmentIdentity::new("registry-bench", "source_lookup", 1, 1, "capability", 1);
    builder.add_type_capabilities(benchmark_target_type(), capabilities, source);
    builder.build().expect("source lookup benchmark snapshot is valid")
}

/// Builds an isolated registry with the requested number of generic source entries.
fn prepare_definition_source_lookup_registry(capability_count: usize) -> (ReflectRegistry, TypeDefinitionId) {
    let definition_id = TypeDefinitionId::of::<Vec<u8>>();
    let generics = Box::leak(Box::new(GenericDefinitionDescriptor::new([], [])));
    let definition = Box::leak(Box::new(TypeDefinitionDescriptor::opaque(
        definition_id,
        "alloc::vec::Vec",
        "Vec",
        generics,
    )));
    let mut builder = RegistrySnapshotBuilder::new();
    let capabilities = (0..capability_count)
        .map(|index| {
            let id: &'static str = Box::leak(format!("registry.bench_definition_source_{index}").into_boxed_str());
            CapabilityDescriptor::without_adapter(CapabilityKey::<()>::new(
                CapabilityId::new(id).expect("benchmark definition capability ID is valid"),
            ))
        })
        .collect();
    let source = FragmentIdentity::new("registry-bench", "definition_source_lookup", 1, 1, "capability", 1);
    builder.add_definition_capabilities(definition, capabilities, source);
    (builder.build().expect("definition source lookup snapshot is valid"), definition_id)
}

static REGISTRY_BENCHMARK_FRAGMENT: RegistrationFragment = RegistrationFragment::new(
    FragmentKind::Type,
    StaticFragmentIdentity::new("registry-bench", "single", 1, 1, "type", 1),
    benchmark_runtime_identity,
    benchmark_payload,
);

reflect::__private::codegen_v3::inventory::submit! {
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

    let mut source_lookup = criterion.benchmark_group("registry/capability_source");
    for capability_count in [1_usize, 100, 10_000] {
        let source_registry = prepare_source_lookup_registry(capability_count);
        let hit_id = format!("registry.bench_source_{}", capability_count - 1);
        source_lookup.bench_function(BenchmarkId::new("hit", capability_count), |bench| {
            bench.iter(|| black_box(source_registry.capability_source(benchmark_target_type(), black_box(&hit_id))));
        });
        source_lookup.bench_function(BenchmarkId::new("miss", capability_count), |bench| {
            bench.iter(|| black_box(source_registry.capability_source(benchmark_target_type(), "registry.bench_missing")));
        });
    }
    source_lookup.finish();

    let mut definition_source_lookup = criterion.benchmark_group("registry/definition_capability_source");
    for capability_count in [1_usize, 100, 10_000] {
        let (source_registry, definition_id) = prepare_definition_source_lookup_registry(capability_count);
        let hit_id = format!("registry.bench_definition_source_{}", capability_count - 1);
        definition_source_lookup.bench_function(BenchmarkId::new("hit", capability_count), |bench| {
            bench.iter(|| black_box(source_registry.definition_capability_source(definition_id, black_box(&hit_id))));
        });
        definition_source_lookup.bench_function(BenchmarkId::new("miss", capability_count), |bench| {
            bench.iter(|| black_box(source_registry.definition_capability_source(definition_id, "registry.bench_missing")));
        });
    }
    definition_source_lookup.finish();

    let mut effective_view = criterion.benchmark_group("registry/effective_view_build");
    for method_count in [0_usize, 2, 9, 18] {
        let implementations = prepare_effective_method_facts(method_count);
        assert_eq!(
            build_benchmark_effective_type_view(&implementations).methods().len(),
            method_count,
        );
        effective_view.bench_with_input(
            BenchmarkId::from_parameter(method_count),
            &implementations,
            |bench, implementations| {
                bench.iter(|| black_box(build_benchmark_effective_type_view(implementations)));
            },
        );
    }
    effective_view.finish();
}

criterion_group!(benches, registry_operations);
criterion_main!(benches);
