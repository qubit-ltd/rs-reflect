// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Direct-versus-reflected baselines for field access, invocation, and
//! construction.

use std::hint::black_box;
use std::time::Instant;

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

/// Measures one operation and prints its elapsed wall-clock duration.
fn measure(name: &str, operation: impl FnOnce()) {
    let started = Instant::now();
    operation();
    println!("{name}: {:?}", started.elapsed());
}

fn main() {
    const ITERATIONS: u64 = 100_000;
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

    let mut direct = BenchmarkRecord { id: 0 };
    measure("direct field get", || {
        for _ in 0..ITERATIONS {
            black_box(direct.id);
        }
    });
    measure("direct field set", || {
        for value in 0..ITERATIONS {
            black_box(&mut direct).id = black_box(value);
        }
    });
    black_box(direct.id);

    let mut reflected = BenchmarkRecord { id: 0 };
    measure("dynamic field get", || {
        for _ in 0..ITERATIONS {
            let _ = black_box(field.get(ReflectedRef::new(&reflected)));
        }
    });
    measure("dynamic field set", || {
        for value in 0..ITERATIONS {
            let _ = black_box(field.set(ReflectedMut::new(&mut reflected), ReflectedOwned::new(black_box(value))));
        }
    });
    assert_eq!(reflected.id, ITERATIONS - 1);

    let mut direct_method = BenchmarkRecord { id: 0 };
    measure("direct method invoke", || {
        for _ in 0..ITERATIONS {
            black_box(direct_method.increment());
        }
    });
    black_box(direct_method.id);

    let mut reflected_method = BenchmarkRecord { id: 0 };
    measure("dynamic method invoke", || {
        for _ in 0..ITERATIONS {
            let _ = black_box(method.invoke_local(Invocation::borrowed_mut(
                DynamicMut::<Local>::new(&mut reflected_method),
                [],
            )));
        }
    });
    assert_eq!(reflected_method.id, ITERATIONS);

    measure("direct construct", || {
        for value in 0..ITERATIONS {
            black_box(BenchmarkRecord { id: value });
        }
    });

    measure("dynamic construct", || {
        for value in 0..ITERATIONS {
            let _ = black_box(
                descriptor.construct_struct(NamedConstructionInput::new([("id", ReflectedOwned::new(value))])),
            );
        }
    });
}
