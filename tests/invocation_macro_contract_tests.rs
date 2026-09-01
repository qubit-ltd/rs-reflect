// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow explicit-imports
//! Macro-level contracts for panic, async borrowing, and invocation modes.

use std::cell::Cell;
use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::panic::catch_unwind;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;

use qubit_reflect as reflect;
use qubit_reflect::Reflect;
use qubit_reflect::descriptor::CatchingAvailability;
use qubit_reflect::descriptor::MethodInstanceDescriptor;
use qubit_reflect::descriptor::MethodLookup;
use qubit_reflect::descriptor::MethodQualifier;
use qubit_reflect::invoke::Invocation;
use qubit_reflect::invoke::InvocationArg;
use qubit_reflect::invoke::InvocationOutput;
use qubit_reflect::reflect_impl;
use qubit_reflect::registry::ReflectRegistry;
use qubit_reflect::value::DynamicOwned;
use qubit_reflect::value::DynamicRef;
use qubit_reflect::value::Local;
use qubit_reflect::value::ThreadSafe;

#[derive(Reflect)]
struct Worker {
    prefix: String,
}

thread_local! {
    static LOCAL_POLLS: Cell<usize> = const { Cell::new(0) };
}

#[reflect_impl]
impl Worker {
    async fn borrowed_local(&self, suffix: &String) -> String {
        LOCAL_POLLS.with(|polls| polls.set(polls.get() + 1));
        format!("{}{}", self.prefix, suffix)
    }

    async fn non_send_local(value: u8) -> u8 {
        let state = Rc::new(Cell::new(value));
        std::future::ready(()).await;
        state.get() + 1
    }

    #[reflect(thread_safe)]
    async fn send_future(polls: Arc<AtomicUsize>) -> usize {
        polls.fetch_add(1, Ordering::SeqCst);
        17
    }

    fn ordinary_panic() {
        panic!("ordinary panic payload")
    }

    #[reflect(catch_unwind)]
    fn catching_panic() {
        panic!("caught panic payload")
    }

    #[reflect(thread_safe, catch_unwind)]
    fn thread_safe_catching_panic() {
        panic!("thread-safe caught payload")
    }
}

fn method(name: &str) -> &'static MethodInstanceDescriptor {
    let registry = ReflectRegistry::initialize().expect("generated fragments must validate");
    let implementations = registry.implementations(Worker::type_descriptor().type_id());
    let MethodLookup::Unique(method) =
        reflect::descriptor::ImplDescriptor::lookup_method(implementations, MethodQualifier::Inherent, name)
    else {
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
    let output = method("borrowed_local")
        .invoke_local(Invocation::borrowed(
            DynamicRef::<Local>::new(&worker),
            [InvocationArg::Ref(DynamicRef::<Local>::new(&suffix))],
        ))
        .expect("the default local adapter must exist")
        .expect("borrow validation must succeed");
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
    let output = method("non_send_local")
        .invoke_local(Invocation::associated([InvocationArg::Owned(
            DynamicOwned::<Local>::new(6_u8),
        )]))
        .expect("the default local adapter must exist")
        .expect("owned validation must succeed");
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
fn test_thread_safe_async_adapter_returns_a_send_future_without_local_capability() {
    let polls = Arc::new(AtomicUsize::new(0));
    let output = method("send_future")
        .invoke_thread_safe(Invocation::associated([InvocationArg::Owned(
            DynamicOwned::<ThreadSafe>::new(Arc::clone(&polls)),
        )]))
        .expect("the explicit thread-safe adapter must exist")
        .expect("owned validation must succeed");
    assert!(
        method("send_future").invoke_local(Invocation::associated([])).is_none(),
        "thread-safe capability must not be inferred as local"
    );
    let InvocationOutput::Future(future) = output else {
        panic!("async invocation must return a reflected future")
    };
    let output = std::thread::spawn(move || {
        let mut future = future;
        poll_once(&mut future)
    })
    .join()
    .expect("the reflected future must be Send");
    let Poll::Ready(InvocationOutput::Owned(value)) = output else {
        panic!("the future must complete on its first explicit poll")
    };
    let Ok(value) = DynamicOwned::<ThreadSafe>::downcast::<usize>(value) else {
        panic!("the output must retain usize")
    };
    assert_eq!(value, 17);
    assert_eq!(polls.load(Ordering::SeqCst), 1);
}

#[test]
fn test_unmarked_and_marked_panics_keep_distinct_capabilities_and_payloads() {
    let ordinary = method("ordinary_panic");
    assert_eq!(
        ordinary.adapter().unwrap().catching_availability(),
        CatchingAvailability::NotRequested
    );
    assert!(ordinary.invoke_catching_local(Invocation::associated([])).is_none());
    assert!(
        ordinary.invoke_thread_safe(Invocation::associated([])).is_none(),
        "thread-safe capability must require its explicit attribute"
    );
    let payload = match catch_unwind(AssertUnwindSafe(|| {
        ordinary
            .invoke_local(Invocation::associated([]))
            .expect("the normal adapter must exist")
            .expect("validation must succeed")
    })) {
        Ok(_) => {
            panic!("ordinary invocation must propagate the original panic")
        }
        Err(payload) => payload,
    };
    assert_eq!(payload.downcast_ref::<&str>(), Some(&"ordinary panic payload"));

    let catching = method("catching_panic");
    assert_eq!(
        catching.adapter().unwrap().catching_availability(),
        CatchingAvailability::Available
    );
    let payload = match catch_unwind(AssertUnwindSafe(|| {
        catching
            .invoke_local(Invocation::associated([]))
            .expect("the normal adapter must remain present")
            .expect("validation must succeed")
    })) {
        Ok(_) => {
            panic!("normal invocation must still propagate the original panic")
        }
        Err(payload) => payload,
    };
    assert_eq!(payload.downcast_ref::<&str>(), Some(&"caught panic payload"));
    let panic = match catching
        .invoke_catching_local(Invocation::associated([]))
        .expect("the explicit catching adapter must exist")
        .expect("validation must succeed")
    {
        Ok(_) => panic!("the user panic must be captured"),
        Err(panic) => panic,
    };
    assert_eq!(panic.payload().downcast_ref::<&str>(), Some(&"caught panic payload"));
}

#[test]
fn test_thread_safe_and_catching_attributes_compose_explicitly() {
    let catching = method("thread_safe_catching_panic");
    assert!(catching.invoke_local(Invocation::associated([])).is_none());
    assert!(catching.invoke_catching_local(Invocation::associated([])).is_none());
    assert_eq!(
        catching.adapter().unwrap().catching_availability(),
        CatchingAvailability::Available
    );
    let panic = match catching
        .invoke_catching_thread_safe(Invocation::associated([]))
        .expect("the explicit thread-safe catching adapter must exist")
        .expect("validation must succeed")
    {
        Ok(_) => panic!("the user panic must be captured"),
        Err(panic) => panic,
    };
    assert_eq!(
        panic.payload().downcast_ref::<&str>(),
        Some(&"thread-safe caught payload")
    );
}
