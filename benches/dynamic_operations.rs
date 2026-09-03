// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Direct-versus-reflected baselines for field access, invocation, and
//! construction.

use criterion::BatchSize;
use criterion::Criterion;
use criterion::black_box;
use criterion::criterion_group;
use criterion::criterion_main;
use qubit_reflect::Reflect;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::construct::NamedConstructionInput;
use qubit_reflect::descriptor::MethodLookup;
use qubit_reflect::invoke::Invocation;
use qubit_reflect::invoke::InvocationOutput;
use qubit_reflect::reflect_impl;
use qubit_reflect::value::DynamicMut;
use qubit_reflect::value::DynamicOwned;
use qubit_reflect::value::Local;
use qubit_reflect::value::ReflectedMut;
use qubit_reflect::value::ReflectedOwned;
use qubit_reflect::value::ReflectedRef;

#[derive(Reflect)]
struct BenchmarkRecord {
    id: u64,
}

#[reflect_impl]
impl BenchmarkRecord {
    fn increment(&mut self) -> u64 {
        self.id += 1;
        self.id
    }
}

/// Registers direct and reflected operation pairs.
fn dynamic_operations(criterion: &mut Criterion) {
    let descriptor = TypeDescriptor::of::<BenchmarkRecord>();
    let field = descriptor.field("id").expect("benchmark field must exist");
    let MethodLookup::Unique(method) = descriptor
        .methods_named("increment")
        .expect("benchmark registry must initialize")
    else {
        panic!("benchmark method must resolve uniquely");
    };

    let mut field_probe = BenchmarkRecord { id: 1 };
    assert_eq!(
        field
            .get(ReflectedRef::new(&field_probe))
            .expect("dynamic field get setup must succeed")
            .downcast_ref::<u64>(),
        Some(&1),
    );
    field
        .set(ReflectedMut::new(&mut field_probe), ReflectedOwned::new(2_u64))
        .expect("dynamic field set setup must succeed");
    assert_eq!(field_probe.id, 2);

    let mut method_probe = BenchmarkRecord { id: 2 };
    let method_probe_output = method
        .invoke_local(Invocation::borrowed_mut(
            DynamicMut::<Local>::new(&mut method_probe),
            [],
        ))
        .expect("dynamic method setup must have an adapter")
        .expect("dynamic method setup must validate");
    let InvocationOutput::Owned(method_probe_value) = method_probe_output else {
        panic!("dynamic method setup must return an owned value");
    };
    assert_eq!(
        DynamicOwned::<Local>::downcast::<u64>(method_probe_value)
            .unwrap_or_else(|_| panic!("dynamic method setup output must retain its type")),
        3,
    );
    descriptor
        .construct_struct(NamedConstructionInput::new([("id", ReflectedOwned::new(3_u64))]))
        .expect("dynamic construction setup must succeed");

    criterion.bench_function("field/direct_get", |bench| {
        let direct = BenchmarkRecord { id: 0 };
        bench.iter(|| black_box(direct.id));
    });
    criterion.bench_function("field/direct_set", |bench| {
        bench.iter_batched(
            || BenchmarkRecord { id: 0 },
            |mut value| black_box(&mut value).id = black_box(1),
            BatchSize::SmallInput,
        );
    });
    criterion.bench_function("field/reflected_get", |bench| {
        let reflected = BenchmarkRecord { id: 0 };
        bench.iter(|| black_box(field.get(ReflectedRef::new(&reflected))));
    });
    criterion.bench_function("field/reflected_set", |bench| {
        bench.iter_batched(
            || BenchmarkRecord { id: 0 },
            |mut value| black_box(field.set(ReflectedMut::new(&mut value), ReflectedOwned::new(black_box(1_u64)))),
            BatchSize::SmallInput,
        );
    });
    criterion.bench_function("method/direct", |bench| {
        bench.iter_batched(
            || BenchmarkRecord { id: 0 },
            |mut value| black_box(value.increment()),
            BatchSize::SmallInput,
        );
    });
    criterion.bench_function("method/reflected", |bench| {
        bench.iter_batched(
            || BenchmarkRecord { id: 0 },
            |mut value| {
                let output = method.invoke_local(Invocation::borrowed_mut(DynamicMut::<Local>::new(&mut value), []));
                black_box(output.is_some());
            },
            BatchSize::SmallInput,
        );
    });
    criterion.bench_function("construction/direct", |bench| {
        bench.iter(|| black_box(BenchmarkRecord { id: black_box(1) }));
    });
    criterion.bench_function("construction/reflected", |bench| {
        bench.iter(|| {
            black_box(descriptor.construct_struct(NamedConstructionInput::new([(
                "id",
                ReflectedOwned::new(black_box(1_u64)),
            )])))
        });
    });
}

criterion_group!(benches, dynamic_operations);
criterion_main!(benches);
