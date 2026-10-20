// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::future::Future;
use std::pin::pin;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;

use qubit_reflect::Invocation;
use qubit_reflect::InvocationOutput;
use qubit_reflect::Reflect;
use qubit_reflect::invoke::CaughtInvocationFailure;
use qubit_reflect::invoke::InvocationErrorKind;
use qubit_reflect::value::Local;

use super::fixtures::Worker;
use super::fixtures::method;

#[Reflect]
struct RecoveryPayload {
    #[reflect(skip)]
    drops: Arc<AtomicUsize>,
}

impl Drop for RecoveryPayload {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn test_preflight_failure_keeps_original_receiver_and_owned_argument() {
    let mut worker = Worker { value: 12 };
    let Err(failure) = method("add").invoke(Invocation::<Local>::on_mut(&mut worker).arg(String::from("keep me")))
    else {
        panic!("the owned String cannot satisfy the i32 parameter");
    };

    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::ArgumentTypeMismatch { index: 0, .. }
    ));
    let Err(retry) = method("add").invoke(failure.into_invocation()) else {
        panic!("recovery must retain the caller's original wrong-typed input");
    };
    assert!(matches!(
        retry.error().kind(),
        InvocationErrorKind::ArgumentTypeMismatch { index: 0, .. }
    ));
    drop(retry);
    assert_eq!(worker.value, 12);
}

#[test]
fn test_named_preflight_failure_retains_binding_for_retry() {
    let worker = Worker::default();
    let Err(failure) = method("sum").invoke(
        Invocation::<Local>::on_ref(&worker)
            .named_arg("left", 2_i32)
            .named_arg("right", String::from("original")),
    ) else {
        panic!("a named argument with the wrong type must be rejected");
    };

    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::ArgumentTypeMismatch { index: 1, .. }
    ));
    let Err(retry) = method("sum").invoke(failure.into_invocation()) else {
        panic!("recovery must preserve the named binding and wrong-typed value");
    };
    assert!(matches!(
        retry.error().kind(),
        InvocationErrorKind::ArgumentTypeMismatch { index: 1, .. }
    ));
}

#[test]
fn test_recovery_owns_the_original_value_until_recovery_is_dropped() {
    let drops = Arc::new(AtomicUsize::new(0));
    let payload = RecoveryPayload {
        drops: Arc::clone(&drops),
    };
    let mut worker = Worker::default();
    let Err(failure) = method("add").invoke(Invocation::<Local>::on_mut(&mut worker).arg(payload)) else {
        panic!("payload type must fail the i32 parameter preflight");
    };

    assert_eq!(drops.load(Ordering::SeqCst), 0);
    let recovered = failure.into_invocation();
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    drop(recovered);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[test]
fn test_rejected_async_invocation_retries_with_recovered_receiver_and_named_arguments() {
    let worker = Worker { value: 17 };
    let Err(failure) = method("async_sum").invoke_catching(
        Invocation::<Local>::on_ref(&worker)
            .named_arg("right", 5_i32)
            .named_arg("left", 3_i32),
    ) else {
        panic!("synchronous catching must reject async methods before execution");
    };
    let CaughtInvocationFailure::Rejected(rejected) = failure else {
        panic!("preflight rejection must remain distinct from a user panic");
    };

    let output = method("async_sum")
        .invoke(rejected.into_invocation())
        .unwrap_or_else(|_| panic!("ordinary dispatch should accept the same recovered call"));
    let InvocationOutput::Future(future) = output else {
        panic!("async methods should return a future");
    };
    let mut future = pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    let Poll::Ready(output) = Future::poll(future.as_mut(), &mut context) else {
        panic!("fixture future should complete on the first poll");
    };

    assert_eq!(output.into_owned::<i32>().unwrap_or_else(|_| panic!("i32 output")), 25);
}
