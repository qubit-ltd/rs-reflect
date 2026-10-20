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
use qubit_reflect::bind_method;
use qubit_reflect::value::Local;
use qubit_reflect::value::ThreadSafe;

#[Reflect]
struct Boxed<T>(T);

#[ReflectImpl]
impl<T: Send + Sync> Boxed<T> {
    fn send_sync_value(&self) -> u32 {
        42
    }
}

#[ReflectImpl]
impl<T> Boxed<T> {
    fn local_value(&self) -> usize {
        7
    }
}

#[test]
fn concrete_send_sync_binding_selects_thread_safe_adapter() {
    let binding = bind_method!(
        Boxed::<u32>::send_sync_value as thread_safe fn(&Boxed<u32>) -> u32
    );
    let context = ReflectionContext::builder()
        .include_linked()
        .bind(binding)
        .build()
        .expect("the concrete generic-impl binding is valid");
    let target = Boxed(11_u32);

    let output = context
        .type_of::<Boxed<u32>>()
        .method("send_sync_value")
        .expect("the concrete method binding is included")
        .invoke(Invocation::<ThreadSafe>::on_ref(&target))
        .unwrap_or_else(|failure| {
            panic!(
                "Boxed<u32> satisfies the binding's ThreadSafe requirements: {:?}",
                failure.error().kind(),
            )
        });

    let value = match output.into_owned::<u32>() {
        Ok(value) => value,
        Err(_) => panic!("the method returns u32"),
    };
    assert_eq!(value, 42);
}

#[test]
fn non_send_generic_target_uses_local_concrete_binding() {
    let binding = bind_method!(Boxed::<Rc<()>>::local_value as fn(&Boxed<Rc<()>>) -> usize);
    let context = ReflectionContext::builder()
        .include_linked()
        .bind(binding)
        .build()
        .expect("the local concrete generic-impl binding is valid");
    let target = Boxed(Rc::new(()));

    let output = context
        .type_of::<Boxed<Rc<()>>>()
        .method("local_value")
        .expect("the concrete method binding is included")
        .invoke(Invocation::<Local>::on_ref(&target))
        .unwrap_or_else(|failure| {
            panic!(
                "a non-Send target remains locally invocable: {:?}",
                failure.error().kind(),
            )
        });

    let value = match output.into_owned::<usize>() {
        Ok(value) => value,
        Err(_) => panic!("the method returns usize"),
    };
    assert_eq!(value, 7);
}
