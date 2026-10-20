// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Covers ThreadSafe adapters for default trait methods with borrowed and
//! structural argument types.

use qubit_reflect::Invocation;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::ReflectionContext;
use qubit_reflect::value::ThreadSafe;

#[Reflect]
struct DefaultParameterTarget;

#[Reflect]
trait DefaultParameterMethods {
    fn describe(&self, name: &String, pair: (u32, u32)) -> (String, u32) {
        (format!("{name}:{}", pair.0 + pair.1), pair.0 * pair.1)
    }

    fn append(&self, value: &mut String) -> usize {
        value.push_str("-updated");
        value.len()
    }
}

#[ReflectImpl(reflected_trait = DefaultParameterMethods)]
impl DefaultParameterMethods for DefaultParameterTarget {}

#[test]
fn test_default_thread_safe_adapter_supports_shared_and_tuple_parameters() {
    let target = DefaultParameterTarget;
    let name = String::from("pair");
    let method = ReflectionContext::global()
        .type_of::<DefaultParameterTarget>()
        .method("describe")
        .expect("the default method is reflected");

    let output = method
        .invoke(
            Invocation::<ThreadSafe>::on_ref(&target)
                .arg_ref(&name)
                .arg((3_u32, 5_u32)),
        )
        .unwrap_or_else(|failure| {
            panic!(
                "the ThreadSafe adapter accepts shared and tuple inputs: {:?}",
                failure.error().kind()
            )
        });

    assert_eq!(
        output
            .into_owned::<(String, u32)>()
            .unwrap_or_else(|_| panic!("describe returns an owned tuple")),
        (String::from("pair:8"), 15),
    );
}

#[test]
fn test_default_thread_safe_adapter_supports_mutable_parameters() {
    let target = DefaultParameterTarget;
    let mut value = String::from("value");
    let method = ReflectionContext::global()
        .type_of::<DefaultParameterTarget>()
        .method("append")
        .expect("the default method is reflected");

    let output = method
        .invoke(Invocation::<ThreadSafe>::on_ref(&target).arg_mut(&mut value))
        .unwrap_or_else(|failure| {
            panic!(
                "the ThreadSafe adapter accepts a mutable String input: {:?}",
                failure.error().kind()
            )
        });

    let length = output
        .into_owned::<usize>()
        .unwrap_or_else(|_| panic!("append returns usize"));
    assert_eq!(value, "value-updated");
    assert_eq!(length, value.len());
}
