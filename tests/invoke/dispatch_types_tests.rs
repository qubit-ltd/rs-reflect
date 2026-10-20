// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::rc::Rc;

use qubit_reflect::Invocation;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::ReflectionContext;
use qubit_reflect::value::Local;
use qubit_reflect::value::ThreadSafe;

use super::fixtures::Worker;
use super::fixtures::local_only_method;
use super::fixtures::local_only_worker;
use super::fixtures::method;

#[Reflect]
struct BorrowedArgumentTarget;

#[ReflectImpl]
impl BorrowedArgumentTarget {
    #[expect(
        clippy::ptr_arg,
        reason = "This fixture reflects the concrete `&String` argument contract."
    )]
    pub fn string_len(&self, input: &String) -> usize {
        input.len()
    }

    pub fn local_rc_count(&self, input: &Rc<()>) -> usize {
        Rc::strong_count(input)
    }
}

fn borrowed_argument_method(name: &str) -> qubit_reflect::MethodDescriptor<'static> {
    ReflectionContext::global()
        .type_of::<BorrowedArgumentTarget>()
        .method(name)
        .expect("the borrowed argument fixture method must be registered")
}

#[test]
fn test_invocation_mode_selects_local_or_thread_safe_adapter() {
    let worker = Worker { value: 8 };

    let local = method("sum")
        .invoke(Invocation::<Local>::on_ref(&worker).arg(1_i32).arg(2_i32))
        .unwrap_or_else(|_| panic!("the local adapter should be available"));
    let shared = method("sum")
        .invoke(Invocation::<ThreadSafe>::on_ref(&worker).arg(1_i32).arg(2_i32))
        .unwrap_or_else(|_| panic!("the thread-safe adapter should be available for a Sync target"));

    assert_eq!(
        local
            .into_owned::<i32>()
            .unwrap_or_else(|_| panic!("local output is i32")),
        11
    );
    assert_eq!(
        shared
            .into_owned::<i32>()
            .unwrap_or_else(|_| panic!("thread-safe output is i32")),
        11
    );
}

#[test]
fn test_non_thread_safe_target_keeps_local_adapter_without_thread_safe_adapter() {
    let worker = local_only_worker(8);

    let local = local_only_method("sum")
        .invoke(Invocation::<Local>::on_ref(&worker).arg(1_i32).arg(2_i32))
        .unwrap_or_else(|failure| panic!("the local adapter remains available: {:?}", failure.error().kind()));
    assert_eq!(
        local
            .into_owned::<i32>()
            .unwrap_or_else(|_| panic!("local output is i32")),
        11
    );
}

#[test]
fn test_thread_safe_adapter_accepts_shared_argument() {
    let target = BorrowedArgumentTarget;
    let input = String::from("shared input");

    let output = borrowed_argument_method("string_len")
        .invoke(Invocation::<ThreadSafe>::on_ref(&target).arg_ref(&input))
        .unwrap_or_else(|failure| {
            panic!(
                "String is Sync and the ThreadSafe adapter accepts its shared borrow: {:?}",
                failure.error().kind()
            )
        });

    assert_eq!(
        output
            .into_owned::<usize>()
            .unwrap_or_else(|_| panic!("string_len returns usize")),
        input.len()
    );
}

#[test]
fn test_non_sync_shared_argument_uses_local_adapter() {
    let target = BorrowedArgumentTarget;
    let input = Rc::new(());

    let output = borrowed_argument_method("local_rc_count")
        .invoke(Invocation::<Local>::on_ref(&target).arg_ref(&input))
        .unwrap_or_else(|failure| {
            panic!(
                "Rc is reflectable for a local shared borrow: {:?}",
                failure.error().kind()
            )
        });

    assert_eq!(
        output
            .into_owned::<usize>()
            .unwrap_or_else(|_| panic!("local_rc_count returns usize")),
        1
    );
}
