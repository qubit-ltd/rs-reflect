// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![no_main]

use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use libfuzzer_sys::fuzz_target;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectedMut;
use qubit_reflect::ReflectedOwned;
use qubit_reflect::ReflectedRef;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::construct::NamedConstructionInput;
use qubit_reflect::descriptor::MethodLookup;
use qubit_reflect::invoke::Invocation;
use qubit_reflect::invoke::InvocationArg;
use qubit_reflect::reflect_impl;
use qubit_reflect::registry::ReflectRegistry;
use qubit_reflect::value::DynamicMut;
use qubit_reflect::value::Local;

const MAX_INPUT_BYTES: usize = 4_096;
static DROPS: AtomicUsize = AtomicUsize::new(0);

#[derive(Reflect)]
struct DynamicRecord {
    value: u64,
}

#[reflect_impl]
impl DynamicRecord {
    fn add(&mut self, delta: u64) {
        self.value = self.value.wrapping_add(delta);
    }
}

#[derive(Reflect)]
struct DropProbe;

impl Drop for DropProbe {
    fn drop(&mut self) {
        DROPS.fetch_add(1, Ordering::SeqCst);
    }
}

/// Resets the probe and verifies one recovered owned value is dropped exactly
/// once by `operation`.
fn assert_one_recovered_drop(operation: impl FnOnce(ReflectedOwned)) {
    DROPS.store(0, Ordering::SeqCst);
    operation(ReflectedOwned::new(DropProbe));
    assert_eq!(DROPS.load(Ordering::SeqCst), 1);
}

fuzz_target!(|unbounded: &[u8]| {
    let data = &unbounded[..unbounded.len().min(MAX_INPUT_BYTES)];
    let registry =
        ReflectRegistry::initialize().expect("the fuzz target's linked fragments must form a valid snapshot");
    let descriptor = TypeDescriptor::of::<DynamicRecord>();
    let registry_type_count = registry.types().len();
    let registered_descriptor = registry
        .get(descriptor.type_id())
        .expect("the derived fuzz target must be registered");
    let field = descriptor.field("value").expect("derived field");
    let MethodLookup::Unique(method) = descriptor.methods_named("add").expect("registry lookup") else {
        panic!("derived method must resolve uniquely");
    };

    let mut record = DynamicRecord { value: 0 };
    for command in data {
        match command % 5 {
            0 => {
                let value = field.get(ReflectedRef::new(&record)).expect("exact shared target");
                assert_eq!(value.downcast_ref::<u64>(), Some(&record.value));
            }
            1 => {
                field
                    .set(ReflectedMut::new(&mut record), ReflectedOwned::new(u64::from(*command)))
                    .expect("exact replacement");
            }
            2 => assert_one_recovered_drop(|replacement| {
                let failure = field
                    .set(ReflectedMut::new(&mut record), replacement)
                    .expect_err("wrong replacement type");
                assert_eq!(DROPS.load(Ordering::SeqCst), 0);
                drop(
                    failure
                        .into_recovery()
                        .expect("pre-execution field failure")
                        .into_value(),
                );
            }),
            3 => assert_one_recovered_drop(|argument| {
                let result = method
                    .invoke_local(Invocation::borrowed_mut(
                        DynamicMut::<Local>::new(&mut record),
                        [InvocationArg::Owned(argument)],
                    ))
                    .expect("generated adapter");
                let Err(failure) = result else {
                    panic!("wrong invocation argument type");
                };
                assert_eq!(DROPS.load(Ordering::SeqCst), 0);
                drop(failure.into_recovery());
            }),
            _ => assert_one_recovered_drop(|value| {
                let result = descriptor.construct_struct(NamedConstructionInput::new([("value", value)]));
                let Err(recovery) = result else {
                    panic!("wrong construction field type");
                };
                assert_eq!(DROPS.load(Ordering::SeqCst), 0);
                drop(recovery);
            }),
        }

        assert_eq!(registry.types().len(), registry_type_count);
        assert!(std::ptr::eq(
            registry
                .get(descriptor.type_id())
                .expect("registry identity remains indexed"),
            registered_descriptor,
        ));
    }
});
