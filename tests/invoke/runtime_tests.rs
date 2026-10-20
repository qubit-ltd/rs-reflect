// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::future::Future;
use std::pin::pin;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;

use qubit_reflect::Invocation;
use qubit_reflect::InvocationOutput;
use qubit_reflect::value::Local;

use super::fixtures::Worker;
use super::fixtures::method;

#[test]
fn test_associated_and_instance_receivers_follow_method_metadata() {
    let worker = Worker { value: 10 };
    let instance = method("sum")
        .invoke(Invocation::<Local>::on_ref(&worker).arg(2_i32).arg(3_i32))
        .unwrap_or_else(|_| panic!("an instance receiver should match the method declaration"));
    assert_eq!(
        instance
            .into_owned::<i32>()
            .unwrap_or_else(|_| panic!("sum output is i32")),
        15
    );

    let Err(failure) = method("sum").invoke(Invocation::<Local>::associated().arg(2_i32).arg(3_i32)) else {
        panic!("associated input must not satisfy a shared receiver");
    };
    assert!(matches!(
        failure.error().kind(),
        qubit_reflect::invoke::InvocationErrorKind::ReceiverMissing
    ));
}

#[test]
fn test_named_and_positional_bindings_can_be_mixed() {
    let worker = Worker { value: 10 };
    let output = method("sum")
        .invoke(
            Invocation::<Local>::on_ref(&worker)
                .arg(2_i32)
                .named_arg("right", 3_i32),
        )
        .unwrap_or_else(|_| panic!("mixed positional and named arguments should bind"));

    assert_eq!(
        output
            .into_owned::<i32>()
            .unwrap_or_else(|_| panic!("sum output is i32")),
        15
    );
}

#[test]
fn test_async_method_returns_a_future_for_the_caller_to_poll() {
    let worker = Worker { value: 41 };
    let output = method("async_value")
        .invoke(Invocation::<Local>::on_ref(&worker))
        .unwrap_or_else(|_| panic!("async dispatch should construct a reflected future"));
    let InvocationOutput::Future(future) = output else {
        panic!("async method dispatch must return InvocationOutput::Future");
    };
    let mut future = pin!(future);
    let mut context = Context::from_waker(Waker::noop());

    let Poll::Ready(InvocationOutput::Owned(value)) = Future::poll(future.as_mut(), &mut context) else {
        panic!("the fixture future should complete on its first poll");
    };
    assert_eq!(
        InvocationOutput::Owned(value)
            .into_owned::<i32>()
            .unwrap_or_else(|_| panic!("future output type")),
        41
    );
}
