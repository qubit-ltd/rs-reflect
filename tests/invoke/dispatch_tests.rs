// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::Invocation;
use qubit_reflect::invoke::InvocationErrorKind;
use qubit_reflect::value::Local;

use super::fixtures::Worker;
use super::fixtures::method;

#[test]
fn test_invocation_rejects_missing_receiver() {
    let Err(failure) = method("add").invoke(Invocation::<Local>::associated().arg(9_i32)) else {
        panic!("an instance method requires a receiver");
    };

    assert!(matches!(failure.error().kind(), InvocationErrorKind::ReceiverMissing));
}

#[test]
fn test_invocation_rejects_wrong_argument_type_before_method_body() {
    let mut worker = Worker::default();
    let Err(failure) = method("add").invoke(Invocation::<Local>::on_mut(&mut worker).arg(String::from("preserved")))
    else {
        panic!("argument TypeId must be checked before dispatch");
    };

    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::ArgumentTypeMismatch { index: 0, .. }
    ));
    let Err(retry) = method("add").invoke(failure.into_invocation()) else {
        panic!("the recovered call must retain its original wrong-typed argument");
    };
    assert!(matches!(
        retry.error().kind(),
        InvocationErrorKind::ArgumentTypeMismatch { index: 0, .. }
    ));
    drop(retry);
    assert_eq!(worker.value, 0);
}

#[test]
fn test_unsafe_retry_uses_same_receiver_and_argument_after_visibility_rejection() {
    let mut worker = Worker::default();
    let Err(private) = method("private_method").invoke(Invocation::<Local>::on_mut(&mut worker).arg(5_i32)) else {
        panic!("private methods require the explicit unsafe API");
    };
    assert!(matches!(private.error().kind(), InvocationErrorKind::VisibilityDenied));
    let private_output = unsafe { method("private_method").invoke_unchecked(private.into_invocation()) }
        .unwrap_or_else(|_| panic!("unchecked retry should use the recovered private call inputs"));
    assert!(matches!(&private_output, qubit_reflect::InvocationOutput::Unit));
    drop(private_output);
    assert_eq!(worker.value, 5);

    let Err(unsafe_method) = method("unsafe_method").invoke(Invocation::<Local>::on_mut(&mut worker).arg(7_i32)) else {
        panic!("unsafe methods require the explicit unsafe API");
    };
    assert!(matches!(
        unsafe_method.error().kind(),
        InvocationErrorKind::UnsafeMethod
    ));
    let unsafe_output = unsafe { method("unsafe_method").invoke_unchecked(unsafe_method.into_invocation()) }
        .unwrap_or_else(|_| panic!("unchecked retry should use the recovered unsafe call inputs"));
    assert!(matches!(&unsafe_output, qubit_reflect::InvocationOutput::Unit));
    drop(unsafe_output);
    assert_eq!(worker.value, 12);
}

#[test]
fn test_named_bindings_dispatch_in_declaration_order() {
    let worker = Worker { value: 1 };
    let output = method("sum")
        .invoke(
            Invocation::<Local>::on_ref(&worker)
                .named_arg("right", 4_i32)
                .named_arg("left", 2_i32),
        )
        .unwrap_or_else(|_| panic!("known named bindings should be reordered by parameter metadata"));

    assert_eq!(
        output.into_owned::<i32>().unwrap_or_else(|_| panic!("sum returns i32")),
        7
    );
}
