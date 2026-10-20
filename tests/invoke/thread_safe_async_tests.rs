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
fn explicit_thread_safe_async_binding_returns_a_send_reflected_future() {
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
    let mut context = Context::from_waker(Waker::noop());
    let Poll::Ready(output) = Future::poll(future.as_mut(), &mut context) else {
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
fn local_async_invocation_accepts_a_non_send_native_future() {
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
    let mut context = Context::from_waker(Waker::noop());
    let Poll::Ready(output) = Future::poll(future.as_mut(), &mut context) else {
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
