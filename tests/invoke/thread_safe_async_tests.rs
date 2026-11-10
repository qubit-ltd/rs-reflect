// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::future::Future;
use std::pin::pin;
use std::rc::Rc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;

use qubit_reflect::Invocation;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::ReflectionContext;
use qubit_reflect::bind_method;
use qubit_reflect::value::Local;
use qubit_reflect::value::ThreadSafe;

#[Reflect]
struct ThreadSafeAsyncTarget {
    _marker: (),
}

#[ReflectImpl]
impl ThreadSafeAsyncTarget {
    pub fn registered(&self) {}

    pub async fn async_value<T: qubit_reflect::__private::codegen_v4::descriptor::Reflect>(&self) -> u32 {
        let _: std::marker::PhantomData<T> = std::marker::PhantomData;
        42
    }
}

fn thread_safe_async_binding()
-> qubit_reflect::__private::codegen_v4::registration::MethodBinding<ThreadSafeAsyncTarget> {
    bind_method!(
        ThreadSafeAsyncTarget::async_value::<u32>
            as thread_safe async fn(&ThreadSafeAsyncTarget) -> u32
    )
}

#[Reflect]
struct LocalAsyncTarget {
    #[reflect(skip)]
    state: Rc<()>,
}

#[ReflectImpl]
impl LocalAsyncTarget {
    pub async fn local_value(&self) -> usize {
        let state = Rc::clone(&self.state);
        std::future::ready(()).await;
        Rc::strong_count(&state)
    }
}

#[test]
fn test_explicit_thread_safe_async_binding_returns_a_send_reflected_future() {
    let context = ReflectionContext::builder()
        .include_linked()
        .bind(thread_safe_async_binding())
        .build()
        .expect("the explicit thread-safe async binding should register");
    let target = ThreadSafeAsyncTarget { _marker: () };
    let method = context
        .type_of::<ThreadSafeAsyncTarget>()
        .method("async_value")
        .expect("the async method should be registered");
    let output = method
        .invoke(Invocation::<ThreadSafe>::on_ref(&target))
        .unwrap_or_else(|error| {
            panic!(
                "the explicitly bound thread-safe adapter should be callable: {}",
                error.error()
            )
        });
    let future = output
        .into_future()
        .unwrap_or_else(|_| panic!("the async adapter returns a reflected future"));
    assert_send(&future);

    let mut future = pin!(future);
    let mut poll_context = Context::from_waker(Waker::noop());
    let Poll::Ready(output) = Future::poll(future.as_mut(), &mut poll_context) else {
        panic!("the fixture future should complete on its first poll");
    };
    assert_eq!(
        output
            .into_owned::<u32>()
            .unwrap_or_else(|_| panic!("the async result should be u32")),
        42
    );
}

#[test]
fn test_local_async_invocation_accepts_a_non_send_native_future() {
    let context = ReflectionContext::builder()
        .include_linked()
        .build()
        .expect("the local async target should register");
    let target = LocalAsyncTarget { state: Rc::new(()) };
    let method = context
        .type_of::<LocalAsyncTarget>()
        .method("local_value")
        .expect("the local async method should be registered");
    let output = method
        .invoke(Invocation::<Local>::on_ref(&target))
        .unwrap_or_else(|_| panic!("the local adapter should accept its non-Send future"));
    let mut future = pin!(
        output
            .into_future()
            .unwrap_or_else(|_| panic!("the async adapter returns a reflected future"))
    );
    let mut poll_context = Context::from_waker(Waker::noop());
    let Poll::Ready(output) = Future::poll(future.as_mut(), &mut poll_context) else {
        panic!("the local fixture future should complete on its first poll");
    };
    assert_eq!(
        output
            .into_owned::<usize>()
            .unwrap_or_else(|_| panic!("the async result should be usize")),
        2
    );
}

fn assert_send<T: Send>(_: &T) {}

#[Reflect]
struct AsyncUnitTarget {
    #[reflect(skip)]
    calls: AtomicUsize,
}

#[ReflectImpl]
impl AsyncUnitTarget {
    pub async fn run_implicit<T: qubit_reflect::__private::codegen_v4::descriptor::Reflect>(&self) {
        let _: std::marker::PhantomData<T> = std::marker::PhantomData;
        self.calls.fetch_add(1, Ordering::SeqCst);
    }

    pub async fn run_implicit_thread_safe<T: qubit_reflect::__private::codegen_v4::descriptor::Reflect>(&self) {
        let _: std::marker::PhantomData<T> = std::marker::PhantomData;
        self.calls.fetch_add(1, Ordering::SeqCst);
    }

    pub async fn run_explicit<T: qubit_reflect::__private::codegen_v4::descriptor::Reflect>(&self) -> () {
        let _: std::marker::PhantomData<T> = std::marker::PhantomData;
        self.calls.fetch_add(1, Ordering::SeqCst);
    }

    pub async fn run_explicit_local<T: qubit_reflect::__private::codegen_v4::descriptor::Reflect>(&self) -> () {
        let _: std::marker::PhantomData<T> = std::marker::PhantomData;
        self.calls.fetch_add(1, Ordering::SeqCst);
    }

    pub async fn pending_value<T: qubit_reflect::__private::codegen_v4::descriptor::Reflect>(&self) -> usize {
        let _: std::marker::PhantomData<T> = std::marker::PhantomData;
        let mut pending = true;
        std::future::poll_fn(|_| {
            if pending {
                pending = false;
                Poll::Pending
            } else {
                Poll::Ready(())
            }
        })
        .await;
        self.calls.fetch_add(1, Ordering::SeqCst);
        7
    }
}

#[test]
fn test_async_unit_binding_runs_only_when_its_future_is_polled() {
    let context = ReflectionContext::builder()
        .include_linked()
        .bind(bind_method!(AsyncUnitTarget::run_implicit::<u32> as async fn(&AsyncUnitTarget)))
        .bind(bind_method!(AsyncUnitTarget::run_implicit_thread_safe::<u32> as thread_safe async fn(&AsyncUnitTarget)))
        .bind(bind_method!(AsyncUnitTarget::run_explicit_local::<u32> as async fn(&AsyncUnitTarget) -> ()))
        .bind(bind_method!(AsyncUnitTarget::run_explicit::<u32> as thread_safe async fn(&AsyncUnitTarget) -> ()))
        .bind(bind_method!(AsyncUnitTarget::pending_value::<u32> as async fn(&AsyncUnitTarget) -> usize))
        .build()
        .expect("the async unit binding should register");
    let target = AsyncUnitTarget {
        calls: AtomicUsize::new(0),
    };
    let method = context.type_of::<AsyncUnitTarget>().method("run_implicit").unwrap();
    let output = method
        .invoke(Invocation::<Local>::on_ref(&target))
        .unwrap_or_else(|_| panic!("the async unit binding should invoke"));
    assert_eq!(target.calls.load(Ordering::SeqCst), 0);
    let mut future = pin!(
        output
            .into_future()
            .unwrap_or_else(|_| panic!("the async unit binding returns a future"))
    );
    assert_eq!(target.calls.load(Ordering::SeqCst), 0);
    let mut poll_context = Context::from_waker(Waker::noop());
    let Poll::Ready(output) = Future::poll(future.as_mut(), &mut poll_context) else {
        panic!("the unit future should complete immediately");
    };
    assert!(output.into_unit().is_ok());
    assert_eq!(target.calls.load(Ordering::SeqCst), 1);

    let method = context
        .type_of::<AsyncUnitTarget>()
        .method("run_implicit_thread_safe")
        .unwrap();
    let output = method
        .invoke(Invocation::<ThreadSafe>::on_ref(&target))
        .unwrap_or_else(|_| panic!("the thread-safe implicit-unit binding should invoke"));
    let mut future = pin!(
        output
            .into_future()
            .unwrap_or_else(|_| panic!("binding returns future"))
    );
    let Poll::Ready(output) = Future::poll(future.as_mut(), &mut poll_context) else {
        panic!("the implicitly unit-typed thread-safe future should complete immediately");
    };
    assert!(output.into_unit().is_ok());
    assert_eq!(target.calls.load(Ordering::SeqCst), 2);

    let method = context
        .type_of::<AsyncUnitTarget>()
        .method("run_explicit_local")
        .unwrap();
    let output = method
        .invoke(Invocation::<Local>::on_ref(&target))
        .unwrap_or_else(|_| panic!("the local explicit-unit binding should invoke"));
    let mut future = pin!(
        output
            .into_future()
            .unwrap_or_else(|_| panic!("binding returns future"))
    );
    let Poll::Ready(output) = Future::poll(future.as_mut(), &mut poll_context) else {
        panic!("the explicitly unit-typed local future should complete immediately");
    };
    assert!(output.into_unit().is_ok());
    assert_eq!(target.calls.load(Ordering::SeqCst), 3);

    let method = context.type_of::<AsyncUnitTarget>().method("run_explicit").unwrap();
    let output = method
        .invoke(Invocation::<ThreadSafe>::on_ref(&target))
        .unwrap_or_else(|_| panic!("the thread-safe async unit binding should invoke"));
    assert_eq!(target.calls.load(Ordering::SeqCst), 3);
    let mut future = pin!(
        output
            .into_future()
            .unwrap_or_else(|_| panic!("thread-safe binding returns future"))
    );
    let Poll::Ready(output) = Future::poll(future.as_mut(), &mut poll_context) else {
        panic!("the thread-safe unit future should complete immediately");
    };
    assert!(output.into_unit().is_ok());
    assert_eq!(target.calls.load(Ordering::SeqCst), 4);

    let method = context.type_of::<AsyncUnitTarget>().method("pending_value").unwrap();
    let output = method
        .invoke(Invocation::<Local>::on_ref(&target))
        .unwrap_or_else(|_| panic!("the pending async binding should invoke"));
    let mut future = pin!(
        output
            .into_future()
            .unwrap_or_else(|_| panic!("binding returns future"))
    );
    assert!(matches!(
        Future::poll(future.as_mut(), &mut poll_context),
        Poll::Pending
    ));
    assert_eq!(target.calls.load(Ordering::SeqCst), 4);
    let Poll::Ready(output) = Future::poll(future.as_mut(), &mut poll_context) else {
        panic!("the second poll should complete the future");
    };
    assert_eq!(
        output
            .into_owned::<usize>()
            .unwrap_or_else(|_| panic!("result should be owned usize")),
        7
    );
    assert_eq!(target.calls.load(Ordering::SeqCst), 5);
}
