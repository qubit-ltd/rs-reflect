// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Macro-level contracts for ordinary invocation, borrowed async inputs, and
//! panic propagation through the v3 method API.

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

use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::descriptor::MethodLookup;
use qubit_reflect::invoke::Invocation;
use qubit_reflect::invoke::InvocationOutput;
use qubit_reflect::registry::ReflectRegistry;
use qubit_reflect::value::DynamicOwned;
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

    #[reflect(catch_unwind)]
    pub fn catching_panic() {
        panic!("caught panic payload")
    }

    fn private_value() -> u8 {
        23
    }
}

fn method(name: &str) -> &'static qubit_reflect::descriptor::MethodInstanceDescriptor {
    let registry = ReflectRegistry::initialize().expect("generated fragments must validate");
    let MethodLookup::Unique(method) = Worker::type_descriptor().methods_named_in(registry, name) else {
        panic!("method `{name}` must be uniquely discoverable")
    };
    method
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
    let registry = ReflectRegistry::initialize().expect("valid fixture registry");
    let output = method("borrowed_local")
        .invoke(registry, Invocation::receiver_ref(&worker).arg_ref(&suffix))
        .expect("public method and matching typed inputs must invoke");
    let InvocationOutput::Future(mut future) = output else {
        panic!("async invocation must return a reflected future")
    };
    assert_eq!(LOCAL_POLLS.with(Cell::get), 0, "the framework must not poll implicitly");
    let Poll::Ready(InvocationOutput::Owned(value)) = poll_once(&mut future) else {
        panic!("the future must complete on its first explicit poll")
    };
    assert_eq!(LOCAL_POLLS.with(Cell::get), 1);
    let Ok(value) = DynamicOwned::<Local>::downcast::<String>(value) else {
        panic!("the output must retain String")
    };
    assert_eq!(value, "hello world");
}

#[test]
fn test_local_async_adapter_accepts_a_non_send_future() {
    let registry = ReflectRegistry::initialize().expect("valid fixture registry");
    let output = method("non_send_local")
        .invoke(registry, Invocation::associated_function().arg(6_u8))
        .expect("the local invocation adapter must accept its ordinary value input");
    let InvocationOutput::Future(mut future) = output else {
        panic!("async invocation must return a reflected future")
    };
    let Poll::Ready(InvocationOutput::Owned(value)) = poll_once(&mut future) else {
        panic!("the future must complete on its first explicit poll")
    };
    let Ok(value) = DynamicOwned::<Local>::downcast::<u8>(value) else {
        panic!("the output must retain u8")
    };
    assert_eq!(value, 7);
}

#[test]
fn test_user_panic_propagates_unchanged_from_dynamic_invocation() {
    let registry = ReflectRegistry::initialize().expect("valid fixture registry");
    let payload = catch_unwind(AssertUnwindSafe(|| {
        let _ = method("ordinary_panic")
            .invoke(registry, Invocation::associated_function())
            .expect("public method must have an invocation adapter");
    }))
    .expect_err("ordinary invocation must propagate the original panic");
    assert_eq!(payload.downcast_ref::<&str>(), Some(&"ordinary panic payload"));
}

#[test]
fn test_local_catching_adapter_retains_the_panic_payload() {
    let registry = ReflectRegistry::initialize().expect("valid fixture registry");
    let result = method("catching_panic")
        .invoke_catching_local(registry, Invocation::associated_function())
        .expect("the explicit local catching adapter must be available")
        .expect("an associated invocation has no input validation failures");
    let Err(panic) = result else {
        panic!("the user method panics")
    };
    assert_eq!(panic.payload().downcast_ref::<&str>(), Some(&"caught panic payload"));
}

#[test]
fn test_private_method_requires_unsafe_visibility_bypass() {
    let registry = ReflectRegistry::initialize().expect("valid fixture registry");
    let method = method("private_value");
    let result = method.invoke(registry, Invocation::associated_function());
    let Err(failure) = result else {
        panic!("safe dynamic invocation must reject a private method")
    };
    assert!(matches!(
        failure.error().kind(),
        qubit_reflect::invoke::InvocationErrorKind::VisibilityDenied
    ));

    let output = unsafe { method.invoke_unsafe(registry, Invocation::associated_function()) }
        .expect("unsafe invocation may bypass the method visibility check");
    let InvocationOutput::Owned(value) = output else {
        panic!("the method returns an owned u8")
    };
    assert_eq!(value.value_type_id(), std::any::TypeId::of::<u8>());
    assert_eq!(
        DynamicOwned::<Local>::downcast::<u8>(value).unwrap_or_else(|_| panic!("the method returns u8")),
        23
    );
}
