// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Macro-level contracts for invocation, borrowed async inputs, and panic
//! behavior through the v4 descriptor API.

#![cfg(feature = "derive")]

use std::cell::Cell;
use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::panic::catch_unwind;
use std::pin::Pin;
use std::rc::Rc;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;

use qubit_reflect::Invocation;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::invoke::CaughtInvocationFailure;
use qubit_reflect::invoke::InvocationErrorKind;
use qubit_reflect::value::Local;

#[Reflect]
struct Worker {
    prefix: String,
}

thread_local! {
    static LOCAL_POLLS: Cell<usize> = const { Cell::new(0) };
}

#[ReflectImpl]
impl Worker {
    pub async fn borrowed_local(&self, suffix: &String) -> String {
        LOCAL_POLLS.with(|polls| polls.set(polls.get() + 1));
        format!("{}{}", self.prefix, suffix)
    }

    pub async fn non_send_local(value: u8) -> u8 {
        let state = Rc::new(Cell::new(value));
        std::future::ready(()).await;
        state.get() + 1
    }

    pub fn ordinary_panic() {
        panic!("ordinary panic payload")
    }

    pub fn catching_panic() {
        panic!("caught panic payload")
    }

    fn private_value() -> u8 {
        23
    }
}

fn method(name: &str) -> qubit_reflect::MethodDescriptor<'static> {
    qubit_reflect::ReflectionContext::global()
        .type_of::<Worker>()
        .method(name)
        .expect("fixture method must be registered")
}

fn poll_once<F: Future + Unpin>(future: &mut F) -> Poll<F::Output> {
    let mut context = Context::from_waker(Waker::noop());
    Pin::new(future).poll(&mut context)
}

#[test]
fn test_local_async_future_is_lazy_and_borrows_receiver_and_parameter() {
    let worker = Worker {
        prefix: String::from("hello "),
    };
    let suffix = String::from("world");
    let output = method("borrowed_local")
        .invoke(Invocation::<Local>::on_ref(&worker).arg_ref(&suffix))
        .unwrap_or_else(|_| panic!("public method and matching typed inputs must invoke"));
    let mut future = output
        .into_future()
        .unwrap_or_else(|_| panic!("async invocation must return a reflected future"));
    assert_eq!(LOCAL_POLLS.with(Cell::get), 0, "invocation must not poll implicitly");
    let Poll::Ready(output) = poll_once(&mut future) else {
        panic!("the future must complete on its first explicit poll")
    };
    assert_eq!(LOCAL_POLLS.with(Cell::get), 1);
    assert_eq!(
        output
            .into_owned::<String>()
            .unwrap_or_else(|_| panic!("the output must retain String")),
        "hello world"
    );
}

#[test]
fn test_local_async_adapter_accepts_a_non_send_future() {
    let output = method("non_send_local")
        .invoke(Invocation::<Local>::associated().arg(6_u8))
        .unwrap_or_else(|_| panic!("the local adapter must accept its ordinary value input"));
    let mut future = output
        .into_future()
        .unwrap_or_else(|_| panic!("async invocation must return a reflected future"));
    let Poll::Ready(output) = poll_once(&mut future) else {
        panic!("the future must complete on its first explicit poll")
    };
    assert_eq!(
        output
            .into_owned::<u8>()
            .unwrap_or_else(|_| panic!("the output must retain u8")),
        7
    );
}

#[test]
fn test_user_panic_propagates_unchanged_from_dynamic_invocation() {
    let payload = catch_unwind(AssertUnwindSafe(|| {
        let _ = method("ordinary_panic")
            .invoke(Invocation::<Local>::associated())
            .unwrap_or_else(|_| panic!("public method must have an invocation adapter"));
    }))
    .expect_err("ordinary invocation must propagate the original panic");
    assert_eq!(payload.downcast_ref::<&str>(), Some(&"ordinary panic payload"));
}

#[test]
fn test_local_catching_adapter_retains_the_panic_payload() {
    let Err(CaughtInvocationFailure::Panicked(panic)) =
        method("catching_panic").invoke_catching(Invocation::<Local>::associated())
    else {
        panic!("explicit catching invocation must retain the panic payload")
    };
    assert_eq!(panic.payload().downcast_ref::<&str>(), Some(&"caught panic payload"));
}

#[test]
fn test_private_method_requires_unsafe_visibility_bypass() {
    let method = method("private_value");
    let Err(failure) = method.invoke(Invocation::<Local>::associated()) else {
        panic!("safe dynamic invocation must reject a private method")
    };
    assert!(matches!(failure.error().kind(), InvocationErrorKind::VisibilityDenied));

    let output = unsafe { method.invoke_unchecked(Invocation::<Local>::associated()) }
        .unwrap_or_else(|_| panic!("unsafe invocation may bypass the method visibility check"));
    assert_eq!(
        output
            .into_owned::<u8>()
            .unwrap_or_else(|_| panic!("the private method returns u8")),
        23
    );
}
