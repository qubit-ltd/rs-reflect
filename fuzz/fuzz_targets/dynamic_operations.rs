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
use qubit_reflect::ReflectImpl;
use qubit_reflect::ReflectionContext;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::construct::NamedFields;
use qubit_reflect::invoke::Invocation;
use qubit_reflect::value::DynamicOwned;
use qubit_reflect::value::Local;

const MAX_INPUT_BYTES: usize = 4_096;
static DROPS: AtomicUsize = AtomicUsize::new(0);

#[Reflect]
struct DynamicRecord {
    pub value: u64,
}

#[ReflectImpl]
impl DynamicRecord {
    fn add(&mut self, delta: u64) {
        self.value = self.value.wrapping_add(delta);
    }
}

#[Reflect]
struct DropProbe;

impl Drop for DropProbe {
    fn drop(&mut self) {
        DROPS.fetch_add(1, Ordering::SeqCst);
    }
}

/// Resets the probe and verifies one recovered owned value is dropped exactly
/// once by `operation`.
fn assert_one_recovered_drop(operation: impl FnOnce(DropProbe)) {
    DROPS.store(0, Ordering::SeqCst);
    operation(DropProbe);
    assert_eq!(DROPS.load(Ordering::SeqCst), 1);
}

fuzz_target!(|unbounded: &[u8]| {
    let data = &unbounded[..unbounded.len().min(MAX_INPUT_BYTES)];
    let context = ReflectionContext::global();
    context
        .initialize()
        .expect("the fuzz target's linked fragments must form a valid context");
    let descriptor = TypeDescriptor::of_in::<DynamicRecord>(context);
    let registry_type_count = context.types().expect("a frozen context must enumerate types").len();
    assert!(
        descriptor
            .is_registered()
            .expect("a frozen context must check registration"),
        "the derived fuzz target must be registered"
    );
    let field = descriptor.field("value").expect("derived field");
    let method = descriptor.method("add").expect("derived method must resolve uniquely");

    let mut record = DynamicRecord { value: 0 };
    for command in data {
        match command % 5 {
            0 => {
                let value = field.get::<DynamicRecord, u64>(&record).expect("exact shared target");
                assert_eq!(*value, record.value);
            }
            1 => {
                field.set(&mut record, u64::from(*command)).expect("exact replacement");
            }
            2 => assert_one_recovered_drop(|replacement| {
                let failure = field.set(&mut record, replacement).expect_err("wrong replacement type");
                assert_eq!(DROPS.load(Ordering::SeqCst), 0);
                let (_error, replacement) = failure.into_parts();
                drop(replacement);
            }),
            3 => assert_one_recovered_drop(|argument| {
                let failure =
                    match method.invoke(
                        Invocation::<Local>::on_mut(&mut record).arg_dynamic(DynamicOwned::<Local>::new(argument)),
                    ) {
                        Ok(_) => panic!("wrong invocation argument type"),
                        Err(failure) => failure,
                    };
                assert_eq!(DROPS.load(Ordering::SeqCst), 0);
                drop(failure.into_invocation());
            }),
            _ => assert_one_recovered_drop(|value| {
                let failure = match descriptor
                    .construction()
                    .construct::<DynamicRecord, _>(NamedFields::<Local>::new().field("value", value))
                {
                    Ok(_) => panic!("wrong construction field type"),
                    Err(failure) => failure,
                };
                assert_eq!(DROPS.load(Ordering::SeqCst), 0);
                drop(failure.into_input());
            }),
        }

        assert_eq!(
            context.types().expect("a frozen context must enumerate types").len(),
            registry_type_count,
        );
        assert!(
            descriptor
                .is_registered()
                .expect("a frozen context must check registration")
        );
    }
});
