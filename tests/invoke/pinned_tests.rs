// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::pin::Pin;

use qubit_reflect::InvocationOutput;
use qubit_reflect::invoke::InvocationErrorKind;
use qubit_reflect::invoke::PinnedInvocation;
use qubit_reflect::value::Local;

use super::fixtures::Worker;
use super::fixtures::method;

#[test]
fn test_pinned_mutable_receiver_invokes_pinned_method() {
    let mut worker = Box::pin(Worker { value: 23 });
    let invocation = PinnedInvocation::<Local, _>::mutable(Pin::as_mut(&mut worker));
    let output = method("pinned_value")
        .invoke_pinned(invocation)
        .unwrap_or_else(|_| panic!("the generated pinned receiver adapter should run"));

    assert_eq!(
        output
            .into_owned::<i32>()
            .unwrap_or_else(|_| panic!("pinned value is i32")),
        23
    );
}

#[test]
fn test_pinned_receiver_failure_preserves_pin_and_argument_count() {
    let mut worker = Box::pin(Worker { value: 29 });
    let before = std::ptr::from_ref(Pin::as_ref(&worker).get_ref());
    let invocation = PinnedInvocation::<Local, _>::mutable(Pin::as_mut(&mut worker));
    let Err(failure) = method("sum").invoke_pinned(invocation) else {
        panic!("a non-pinned method must reject a pinned receiver");
    };
    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::ReceiverModeMismatch
    ));

    let output = method("pinned_value")
        .invoke_pinned(failure.into_invocation())
        .unwrap_or_else(|_| panic!("the recovered Pin receiver and empty argument list must retry"));
    assert!(matches!(&output, InvocationOutput::Owned(_)));
    assert_eq!(
        output
            .into_owned::<i32>()
            .unwrap_or_else(|_| panic!("pinned value is i32")),
        29
    );
    assert_eq!(std::ptr::from_ref(Pin::as_ref(&worker).get_ref()), before);
}
