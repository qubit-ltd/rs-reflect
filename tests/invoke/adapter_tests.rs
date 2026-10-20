// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::Invocation;
use qubit_reflect::InvocationOutput;
use qubit_reflect::value::Local;

use super::fixtures::Worker;
use super::fixtures::method;

#[test]
fn test_method_descriptor_invokes_generated_local_adapter() {
    let mut worker = Worker::default();
    let output = method("add")
        .invoke(Invocation::<Local>::on_mut(&mut worker).arg(7_i32))
        .unwrap_or_else(|_| panic!("the generated method adapter should run"));

    assert!(matches!(&output, InvocationOutput::Unit));
    drop(output);
    assert_eq!(worker.value, 7);
}

#[test]
fn test_method_descriptor_returns_owned_output_from_generated_adapter() {
    let worker = Worker { value: 5 };
    let output = method("sum")
        .invoke(Invocation::<Local>::on_ref(&worker).arg(3_i32).arg(4_i32))
        .unwrap_or_else(|_| panic!("the generated method adapter should run"));

    assert_eq!(
        output.into_owned::<i32>().unwrap_or_else(|_| panic!("sum returns i32")),
        12
    );
}
