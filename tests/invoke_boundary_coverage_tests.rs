// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![cfg(feature = "derive")]

use std::pin::Pin;

use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::ReflectionContext;
use qubit_reflect::invoke::CaughtInvocationFailure;
use qubit_reflect::invoke::Invocation;
use qubit_reflect::invoke::InvocationErrorKind;
use qubit_reflect::invoke::InvocationReceiver;
use qubit_reflect::value::DynamicRef;
use qubit_reflect::value::Local;

#[derive(Default)]
#[Reflect]
struct BoundaryTarget {
    value: i32,
}

#[Reflect]
struct OtherReceiver;

#[ReflectImpl]
impl BoundaryTarget {
    pub fn shared(&self, left: i32, right: i32) -> i32 {
        self.value + left + right
    }

    pub fn borrowed_argument(&self, value: &i32) -> i32 {
        self.value + value
    }

    pub fn mutable(&mut self, amount: i32) {
        self.value += amount;
    }

    pub fn associated(value: i32) -> i32 {
        value * 2
    }

    pub fn destructured(&self, (left, right): (i32, i32)) -> i32 {
        self.value + left + right
    }

    pub fn consume(self) -> i32 {
        self.value
    }

    pub fn pinned(self: Pin<&mut Self>) -> i32 {
        self.value
    }

    pub fn borrowed_return(&self) -> &i32 {
        &self.value
    }

    pub fn panic_with_text(&self) {
        panic!("boundary panic payload");
    }

    #[allow(dead_code)]
    pub fn generic<T>(&self, value: T) -> usize {
        std::mem::size_of_val(&value)
    }
}

fn method(name: &str) -> qubit_reflect::MethodDescriptor<'static> {
    ReflectionContext::global()
        .type_of::<BoundaryTarget>()
        .method(name)
        .expect("boundary fixture method must be registered")
}

#[test]
fn invocation_preflight_checks_receiver_shape_and_recoverable_arguments() {
    let target = BoundaryTarget { value: 3 };

    let Err(missing) = method("shared").invoke(Invocation::<Local>::associated().arg(1_i32).arg(2_i32)) else {
        panic!("instance methods must reject a missing receiver");
    };
    assert!(matches!(missing.error().kind(), InvocationErrorKind::ReceiverMissing));

    let Err(unexpected) = method("associated").invoke(Invocation::<Local>::on_ref(&target).arg(2_i32)) else {
        panic!("associated methods must reject an instance receiver");
    };
    assert!(matches!(
        unexpected.error().kind(),
        InvocationErrorKind::UnexpectedReceiver
    ));

    let other_receiver = OtherReceiver;
    let Err(wrong_type) = method("shared").invoke(
        Invocation::<Local>::on_dynamic(InvocationReceiver::Ref(DynamicRef::new(&other_receiver)))
            .arg(1_i32)
            .arg(2_i32),
    ) else {
        panic!("receiver TypeId must match the concrete reflected target");
    };
    assert!(matches!(
        wrong_type.error().kind(),
        InvocationErrorKind::ReceiverTypeMismatch { .. }
    ));

    let target = BoundaryTarget::default();
    let Err(wrong_mode) = method("mutable").invoke(Invocation::<Local>::on_ref(&target).arg(4_i32)) else {
        panic!("mutable receivers must reject shared invocation inputs");
    };
    assert!(matches!(
        wrong_mode.error().kind(),
        InvocationErrorKind::ReceiverModeMismatch
    ));

    let Err(missing_argument) = method("shared").invoke(Invocation::<Local>::on_ref(&target).arg(1_i32)) else {
        panic!("all declared parameters must be bound");
    };
    assert!(matches!(
        missing_argument.error().kind(),
        InvocationErrorKind::MissingArgument { index: 1 }
    ));

    let Err(too_many) = method("shared").invoke(Invocation::<Local>::on_ref(&target).arg(1_i32).arg(2_i32).arg(3_i32))
    else {
        panic!("extra positional inputs must be rejected before dispatch");
    };
    assert!(matches!(
        too_many.error().kind(),
        InvocationErrorKind::TooManyArguments { expected: 2, actual: 3 }
    ));

    let Err(unknown_name) = method("shared").invoke(Invocation::<Local>::on_ref(&target).named_arg("absent", 1_i32))
    else {
        panic!("unknown named arguments must be rejected");
    };
    assert!(
        matches!(unknown_name.error().kind(), InvocationErrorKind::UnknownArgumentName { name } if name.as_ref() == "absent")
    );

    let Err(duplicate_name) = method("shared").invoke(
        Invocation::<Local>::on_ref(&target)
            .named_arg("left", 1_i32)
            .named_arg("left", 2_i32),
    ) else {
        panic!("a parameter cannot be bound twice");
    };
    assert!(
        matches!(duplicate_name.error().kind(), InvocationErrorKind::DuplicateArgumentBinding { name } if name.as_ref() == "left")
    );

    let Err(named_pattern) =
        method("destructured").invoke(Invocation::<Local>::on_ref(&target).named_arg("pair", (1_i32, 2_i32)))
    else {
        panic!("patterns do not expose a bindable parameter name");
    };
    assert!(
        matches!(named_pattern.error().kind(), InvocationErrorKind::NamedArgumentUnavailable { name } if name.as_ref() == "pair")
    );

    let Err(mode_mismatch) = method("borrowed_argument").invoke(Invocation::<Local>::on_ref(&target).arg(5_i32)) else {
        panic!("a shared parameter must reject an owned argument");
    };
    assert!(matches!(
        mode_mismatch.error().kind(),
        InvocationErrorKind::ArgumentModeMismatch { index: 0 }
    ));

    let Err(type_mismatch) = method("shared").invoke(
        Invocation::<Local>::on_ref(&target)
            .arg(String::from("wrong"))
            .arg(2_i32),
    ) else {
        panic!("argument TypeId must match parameter metadata");
    };
    assert!(matches!(
        type_mismatch.error().kind(),
        InvocationErrorKind::ArgumentTypeMismatch { index: 0, .. }
    ));
    let Err(retried_type_mismatch) = method("shared").invoke(type_mismatch.into_invocation()) else {
        panic!("recovered arguments must remain available for a retry");
    };
    assert!(matches!(
        retried_type_mismatch.error().kind(),
        InvocationErrorKind::ArgumentTypeMismatch { index: 0, .. }
    ));

    let mut pinned_target = BoundaryTarget::default();
    let Err(pinned_requires_pinned_api) = method("pinned").invoke(Invocation::<Local>::on_mut(&mut pinned_target))
    else {
        panic!("pinned receiver methods must use the pinned invocation API");
    };
    assert!(matches!(
        pinned_requires_pinned_api.error().kind(),
        InvocationErrorKind::ReceiverAdapterUnavailable
    ));

    let output = method("consume")
        .invoke(Invocation::<Local>::on_owned(BoundaryTarget { value: 3 }))
        .unwrap_or_else(|_| panic!("an owned self method accepts an owned receiver"));
    assert_eq!(
        output
            .into_owned::<i32>()
            .unwrap_or_else(|_| panic!("consume returns i32")),
        3
    );
}

#[test]
fn invocation_dispatch_covers_reordering_outputs_panics_and_unavailable_specializations() {
    let target = BoundaryTarget { value: 10 };
    let output = method("shared")
        .invoke(
            Invocation::<Local>::on_ref(&target)
                .named_arg("right", 4_i32)
                .named_arg("left", 2_i32),
        )
        .unwrap_or_else(|_| panic!("valid named arguments should be reordered to declaration order"));
    assert_eq!(
        output
            .into_owned::<i32>()
            .unwrap_or_else(|_| panic!("shared returns i32")),
        16
    );

    let output = method("associated")
        .invoke(Invocation::<Local>::associated().arg(6_i32))
        .unwrap_or_else(|_| panic!("associated invocation should not need a receiver"));
    assert_eq!(
        output
            .into_owned::<i32>()
            .unwrap_or_else(|_| panic!("associated returns i32")),
        12
    );

    let mut mutable_target = BoundaryTarget::default();
    let output = method("mutable")
        .invoke(Invocation::<Local>::on_mut(&mut mutable_target).arg(7_i32))
        .unwrap_or_else(|_| panic!("valid mutable invocation should dispatch"));
    assert!(matches!(output, qubit_reflect::invoke::InvocationOutput::Unit));

    let returned = method("borrowed_return")
        .invoke(Invocation::<Local>::on_ref(&target))
        .unwrap_or_else(|_| panic!("a receiver-borrowed return should retain its source lifetime"));
    assert_eq!(
        *returned
            .into_ref::<i32>()
            .unwrap_or_else(|_| panic!("borrowed return is i32")),
        10
    );

    let Err(generic) = method("generic").invoke(Invocation::<Local>::on_ref(&target).arg(1_u8)) else {
        panic!("generic methods require a selected specialization");
    };
    assert!(matches!(
        generic.error().kind(),
        InvocationErrorKind::SpecializationRequired
    ));

    let Err(CaughtInvocationFailure::Panicked(panic)) =
        method("panic_with_text").invoke_catching(Invocation::<Local>::on_ref(&target))
    else {
        panic!("catching invocation should distinguish user panics from preflight rejection");
    };
    assert_eq!(panic.message(), Some("boundary panic payload"));
    assert_eq!(panic.payload().downcast_ref::<&str>(), Some(&"boundary panic payload"));
}
