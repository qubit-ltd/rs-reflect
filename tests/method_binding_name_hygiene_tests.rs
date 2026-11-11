// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![cfg(feature = "derive")]

mod shadowed_names {
    #![allow(dead_code)]

    use qubit_reflect::Reflect;
    use qubit_reflect::register_method_binding;

    pub struct Option;
    pub struct Some;
    pub struct None;
    pub struct Result;
    pub struct Ok;
    pub struct Err;
    pub struct Vec;

    #[Reflect]
    pub struct Owner {
        pub count: u32,
    }

    impl Owner {
        pub fn add(&mut self, by: u32) -> u32 {
            self.count += by;
            self.count
        }

        pub fn associated() -> u32 {
            17
        }
    }

    #[Reflect]
    pub struct ThreadSafeOwner;

    impl ThreadSafeOwner {
        pub fn value(&self) -> u32 {
            23
        }
    }

    #[Reflect]
    pub struct RegisteredOwner;

    impl RegisteredOwner {
        pub fn value(&self) -> u32 {
            31
        }
    }

    register_method_binding!(RegisteredOwner::value as fn(&RegisteredOwner) -> u32);
}

use qubit_reflect::Invocation;
use qubit_reflect::ReflectionContext;
use qubit_reflect::bind_method;
use qubit_reflect::value::Local;
use qubit_reflect::value::ThreadSafe;

#[test]
fn test_explicit_method_bindings_work_with_shadowed_standard_names() {
    let binding = bind_method!(shadowed_names::Owner::add as fn(&mut shadowed_names::Owner, u32) -> u32);
    let associated = bind_method!(shadowed_names::Owner::associated as fn() -> u32);
    let context = ReflectionContext::builder()
        .include_type::<shadowed_names::Owner>()
        .bind(binding)
        .bind(associated)
        .build()
        .expect("explicit context with method bindings");

    let mut owner = shadowed_names::Owner { count: 2 };
    let method = context
        .type_of::<shadowed_names::Owner>()
        .method("add")
        .expect("mutable method is registered");
    let output = method
        .invoke(Invocation::<Local>::on_mut(&mut owner).arg(3_u32))
        .unwrap_or_else(|failure| panic!("method invocation failed: {:?}", failure.error().kind()));
    let output = output
        .into_owned::<u32>()
        .unwrap_or_else(|_| panic!("method returns u32"));
    assert_eq!(output, 5);
    assert_eq!(owner.count, 5);

    let output = context
        .type_of::<shadowed_names::Owner>()
        .method("associated")
        .expect("associated method is registered")
        .invoke(Invocation::<Local>::associated())
        .unwrap_or_else(|_| panic!("associated method invocation"));
    let output = output
        .into_owned::<u32>()
        .unwrap_or_else(|_| panic!("associated method returns u32"));
    assert_eq!(output, 17);
}

#[test]
fn test_thread_safe_method_binding_works_with_shadowed_standard_names() {
    let binding =
        bind_method!(shadowed_names::ThreadSafeOwner::value as thread_safe fn(&shadowed_names::ThreadSafeOwner) -> u32);
    let context = ReflectionContext::builder()
        .include_type::<shadowed_names::ThreadSafeOwner>()
        .bind(binding)
        .build()
        .expect("ThreadSafe context with method binding");
    let owner = shadowed_names::ThreadSafeOwner;
    let output = context
        .type_of::<shadowed_names::ThreadSafeOwner>()
        .method("value")
        .expect("ThreadSafe method is registered")
        .invoke(Invocation::<ThreadSafe>::on_ref(&owner))
        .unwrap_or_else(|_| panic!("ThreadSafe method invocation"));
    let output = output
        .into_owned::<u32>()
        .unwrap_or_else(|_| panic!("method returns u32"));
    assert_eq!(output, 23);
}

#[test]
fn test_registered_method_binding_works_with_shadowed_standard_names() {
    let context = ReflectionContext::builder()
        .include_linked()
        .build()
        .expect("linked method binding context");
    let owner = shadowed_names::RegisteredOwner;
    let output = context
        .type_of::<shadowed_names::RegisteredOwner>()
        .method("value")
        .expect("registered method is discovered")
        .invoke(Invocation::<Local>::on_ref(&owner))
        .unwrap_or_else(|_| panic!("registered method invocation"));
    let output = output
        .into_owned::<u32>()
        .unwrap_or_else(|_| panic!("method returns u32"));
    assert_eq!(output, 31);
}
