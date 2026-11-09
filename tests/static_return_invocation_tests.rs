// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![cfg(feature = "derive")]

use qubit_reflect::Invocation;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::invoke::InvocationErrorKind;
use qubit_reflect::invoke::InvocationUnavailableReason;
use qubit_reflect::value::Local;

#[Reflect]
struct StaticTarget;

fn identity(value: &u8) -> u8 {
    *value
}

#[ReflectImpl]
impl StaticTarget {
    pub fn static_pair(&self) -> (&'static str, u8) {
        ("ready", 17)
    }

    pub fn static_option(&self) -> Option<&'static str> {
        Some("optional")
    }

    pub fn static_function(&self) -> for<'a> fn(&'a u8) -> u8 {
        identity
    }

    #[expect(
        dead_code,
        reason = "The fixture checks that external lifetimes cannot use owned output."
    )]
    pub fn explicit_lifetime_pair<'a>(&self, value: &'a str) -> (&'a str, u8) {
        (value, 29)
    }

    #[expect(dead_code, reason = "The fixture checks that opaque output stays unavailable.")]
    pub fn opaque_output(&self) -> impl std::fmt::Display {
        String::from("opaque")
    }

    #[expect(
        dead_code,
        reason = "The fixture checks that receiver borrows cannot escape in a tuple."
    )]
    pub fn receiver_borrow_pair(&self) -> (&str, u8) {
        ("receiver", 19)
    }
}

fn method(name: &str) -> qubit_reflect::MethodDescriptor<'static> {
    qubit_reflect::ReflectionContext::global()
        .type_of::<StaticTarget>()
        .method(name)
        .expect("fixture method must be registered")
}

#[test]
fn test_static_references_nested_in_aggregates_are_invocable_as_owned_values() {
    let target = StaticTarget;
    let output = method("static_pair")
        .invoke(Invocation::<Local>::on_ref(&target))
        .unwrap_or_else(|_| panic!("a static reference inside a tuple is safe to own"));
    assert_eq!(
        output
            .into_owned::<(&'static str, u8)>()
            .unwrap_or_else(|_| panic!("tuple result should retain its static reference")),
        ("ready", 17)
    );
}

#[test]
fn test_static_optional_reference_is_invocable_and_preserves_value() {
    let target = StaticTarget;
    let output = method("static_option")
        .invoke(Invocation::<Local>::on_ref(&target))
        .unwrap_or_else(|_| panic!("Option of a static reference is safe to own"));
    assert_eq!(
        output
            .into_owned::<Option<&'static str>>()
            .unwrap_or_else(|_| panic!("optional result should retain its static reference")),
        Some("optional")
    );
}

#[test]
fn test_aggregate_capturing_receiver_borrow_remains_unavailable() {
    let target = StaticTarget;
    let failure = match method("receiver_borrow_pair").invoke(Invocation::<Local>::on_ref(&target)) {
        Ok(_) => panic!("a tuple containing a receiver borrow cannot escape as owned data"),
        Err(failure) => failure,
    };
    assert_eq!(
        failure.error().kind(),
        &InvocationErrorKind::MethodUnavailable {
            reasons: &[InvocationUnavailableReason::UnsupportedBorrowedReturn],
        }
    );
}

#[test]
fn test_explicit_external_lifetime_return_does_not_escape_through_owned_output() {
    let target = StaticTarget;
    let input = String::from("borrowed");
    let failure =
        match method("explicit_lifetime_pair").invoke(Invocation::<Local>::on_ref(&target).arg_ref(input.as_str())) {
            Ok(_) => panic!("an explicit external lifetime cannot escape through owned output"),
            Err(failure) => failure,
        };
    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::SpecializationRequired | InvocationErrorKind::MethodUnavailable { .. }
    ));
}

#[test]
fn test_opaque_output_remains_unavailable_for_owned_invocation() {
    let target = StaticTarget;
    let failure = match method("opaque_output").invoke(Invocation::<Local>::on_ref(&target)) {
        Ok(_) => panic!("an opaque output cannot use the owned output adapter"),
        Err(failure) => failure,
    };
    assert_eq!(
        failure.error().kind(),
        &InvocationErrorKind::MethodUnavailable {
            reasons: &[InvocationUnavailableReason::OpaqueReturn],
        }
    );
}

#[test]
fn test_higher_ranked_function_pointer_return_is_invocable() {
    let target = StaticTarget;
    let output = method("static_function")
        .invoke(Invocation::<Local>::on_ref(&target))
        .unwrap_or_else(|_| panic!("a function pointer with locally bound lifetimes is safe to own"));
    let function = output
        .into_owned::<for<'a> fn(&'a u8) -> u8>()
        .unwrap_or_else(|_| panic!("function pointer result should preserve its higher-ranked signature"));
    assert_eq!(function(&23), 23);
}
