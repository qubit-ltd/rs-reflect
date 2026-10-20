// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![cfg(feature = "derive")]

use qubit_reflect::Invocation;
use qubit_reflect::InvocationOutput;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::value::Local;
use qubit_reflect::value::ThreadSafe;

#[Reflect]
struct CoverageTarget {
    label: String,
    delta: i32,
}

#[ReflectImpl]
impl CoverageTarget {
    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn borrowed_len(&self, value: &str) -> usize {
        value.len()
    }

    pub fn adjust(&mut self, value: &mut i32) {
        *value += self.delta;
    }

    pub fn combine(&self, left: i32, right: i32) -> i32 {
        left + right + self.delta
    }
}

fn method(name: &str) -> qubit_reflect::MethodDescriptor<'static> {
    qubit_reflect::ReflectionContext::global()
        .type_of::<CoverageTarget>()
        .method(name)
        .expect("coverage fixture method must be registered")
}

#[test]
fn test_invocation_borrowed_output_keeps_receiver_lifetime() {
    let target = CoverageTarget {
        label: String::from("borrowed label"),
        delta: 0,
    };
    let output = method("label")
        .invoke(Invocation::<Local>::on_ref(&target))
        .unwrap_or_else(|failure| {
            panic!(
                "shared receiver return should be callable: {:?}",
                failure.error().kind()
            )
        });

    assert_eq!(
        output.into_ref::<str>().unwrap_or_else(|_| panic!("label returns str")),
        "borrowed label"
    );
}

#[test]
fn test_invocation_borrowed_argument_preserves_call_lifetime() {
    let target = CoverageTarget {
        label: String::new(),
        delta: 0,
    };
    let text = String::from("borrowed input");
    let output = method("borrowed_len")
        .invoke(Invocation::<Local>::on_ref(&target).arg_ref(text.as_str()))
        .unwrap_or_else(|failure| {
            panic!(
                "shared str argument should match the declared borrow mode: {:?}",
                failure.error().kind()
            )
        });

    assert_eq!(
        output
            .into_owned::<usize>()
            .unwrap_or_else(|_| panic!("length is usize")),
        text.len()
    );
}

#[test]
fn test_invocation_mutable_argument_changes_caller_value() {
    let mut target = CoverageTarget {
        label: String::new(),
        delta: 6,
    };
    let mut value = 11_i32;
    let output = method("adjust")
        .invoke(Invocation::<Local>::on_mut(&mut target).arg_mut(&mut value))
        .unwrap_or_else(|failure| {
            panic!(
                "mutable receiver and argument should dispatch together: {:?}",
                failure.error().kind()
            )
        });

    assert!(matches!(&output, InvocationOutput::Unit));
    drop(output);
    assert_eq!(value, 17);
}

#[test]
fn test_invocation_mode_controls_thread_safe_adapter_selection() {
    let target = CoverageTarget {
        label: String::from("safe"),
        delta: 4,
    };
    let local = method("combine")
        .invoke(Invocation::<Local>::on_ref(&target).arg(3_i32).arg(5_i32))
        .unwrap_or_else(|failure| panic!("local invocation should be available: {:?}", failure.error().kind()));
    let thread_safe = method("combine")
        .invoke(Invocation::<ThreadSafe>::on_ref(&target).arg(3_i32).arg(5_i32))
        .unwrap_or_else(|failure| {
            panic!(
                "String and integer fields are Send + Sync: {:?}",
                failure.error().kind()
            )
        });

    assert_eq!(
        local
            .into_owned::<i32>()
            .unwrap_or_else(|_| panic!("local output is i32")),
        12
    );
    assert_eq!(
        thread_safe
            .into_owned::<i32>()
            .unwrap_or_else(|_| panic!("thread-safe output is i32")),
        12
    );
}

#[test]
fn test_named_bindings_keep_parameter_identity_after_reordering() {
    let target = CoverageTarget {
        label: String::new(),
        delta: 2,
    };
    let output = method("combine")
        .invoke(
            Invocation::<Local>::on_ref(&target)
                .named_arg("right", 9_i32)
                .named_arg("left", 1_i32),
        )
        .unwrap_or_else(|failure| {
            panic!(
                "parameter names should select the matching declaration slots: {:?}",
                failure.error().kind()
            )
        });

    assert_eq!(
        output.into_owned::<i32>().unwrap_or_else(|_| panic!("output is i32")),
        12
    );
}
