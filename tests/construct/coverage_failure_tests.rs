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
use qubit_reflect::TypeDescriptor;
use qubit_reflect::construct::ConstructionError;
use qubit_reflect::construct::NamedFields;
use qubit_reflect::construct::PositionalFields;
use qubit_reflect::construct::RecoveredConstructionInput;
use qubit_reflect::invoke::InvocationErrorKind;
use qubit_reflect::value::Local;

#[Reflect]
#[derive(Debug)]
struct TupleRecord(pub u32, pub String);

#[Reflect]
#[derive(Debug)]
pub enum Choice {
    Pair(u32, String),
}

#[Reflect]
#[derive(Debug)]
struct InvocationTarget {
    #[reflect(skip)]
    _local_state: Rc<()>,
    value: i32,
}

#[Reflect]
struct OtherTarget;

#[ReflectImpl]
impl InvocationTarget {
    pub fn combine(&self, left: i32, right: i32) -> i32 {
        self.value + left + right
    }

    pub fn adjust(&mut self, value: &mut i32) {
        *value += self.value;
    }

    fn private_call(&self, value: i32) -> i32 {
        self.value + value
    }
}

fn method(name: &str) -> qubit_reflect::MethodDescriptor<'static> {
    ReflectionContext::global()
        .type_of::<InvocationTarget>()
        .method(name)
        .expect("coverage fixture method is registered")
}

#[test]
fn positional_struct_and_variant_arity_failures_retain_original_inputs() {
    let tuple = TypeDescriptor::of::<TupleRecord>();
    let Err(failure) = tuple.construct::<TupleRecord, _>(
        PositionalFields::<Local>::new()
            .value(1_u32)
            .value(String::from("two"))
            .value(3_u8),
    ) else {
        panic!("tuple structs reject values after the declared fields");
    };
    assert!(
        matches!(
            failure.error(),
            ConstructionError::TooManyValues {
                expected_max: 2,
                actual: 3
            }
        ),
        "unexpected tuple struct error: {:?}",
        failure.error()
    );
    let RecoveredConstructionInput::Positional(input) = failure.into_input() else {
        panic!("a positional arity failure retains positional input");
    };
    let Err(retry) = tuple.construct::<TupleRecord, _>(input) else {
        panic!("retry preserves all values and the original arity error");
    };
    assert!(matches!(
        retry.error(),
        ConstructionError::TooManyValues {
            expected_max: 2,
            actual: 3
        }
    ));

    let pair = TypeDescriptor::of::<Choice>()
        .variant("Pair")
        .expect("tuple variant exists");
    let Err(failure) = pair.construct::<Choice, _>(
        PositionalFields::<Local>::new()
            .value(1_u32)
            .value(String::from("two"))
            .value(3_u8),
    ) else {
        panic!("tuple variants reject excess positional values");
    };
    assert!(
        matches!(
            failure.error(),
            ConstructionError::TooManyValues {
                expected_max: 2,
                actual: 3
            }
        ),
        "unexpected tuple variant error: {:?}",
        failure.error()
    );
    assert!(matches!(
        failure.into_input(),
        RecoveredConstructionInput::Positional(_)
    ));
}

#[test]
fn variant_type_errors_and_unavailable_schema_keep_caller_data() {
    let pair = TypeDescriptor::of::<Choice>()
        .variant("Pair")
        .expect("tuple variant exists");
    let Err(failure) = pair.construct::<Choice, _>(
        PositionalFields::<Local>::new()
            .value(String::from("wrong first field"))
            .value(String::from("second field")),
    ) else {
        panic!("variant fields are checked against their declared types");
    };
    assert!(
        matches!(failure.error(), ConstructionError::ValueTypeMismatch { field: "0", .. }),
        "unexpected tuple variant type error: {:?}",
        failure.error()
    );
    let RecoveredConstructionInput::Positional(input) = failure.into_input() else {
        panic!("variant type failure retains its positional values");
    };
    let Err(retry) = pair.construct::<Choice, _>(input) else {
        panic!("the recovered variant input can be retried");
    };
    assert!(matches!(
        retry.error(),
        ConstructionError::ValueTypeMismatch { field: "0", .. }
    ));

    let unavailable = TypeDescriptor::of::<InvocationTarget>();
    let Err(failure) = unavailable.construct::<InvocationTarget, _>(NamedFields::<Local>::new().field("value", 8_i32))
    else {
        panic!("a skipped field prevents safe construction");
    };
    assert!(matches!(failure.error(), ConstructionError::Unavailable(_)));
    assert!(matches!(failure.into_input(), RecoveredConstructionInput::Named(_)));
}

#[test]
fn invocation_binding_schema_errors_recover_the_original_call() {
    let target = InvocationTarget {
        _local_state: Rc::new(()),
        value: 4,
    };

    let Err(failure) = method("combine").invoke(
        Invocation::<Local>::on_ref(&target)
            .named_arg("not_a_parameter", 1_i32)
            .named_arg("right", 2_i32),
    ) else {
        panic!("unknown argument names are rejected");
    };
    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::UnknownArgumentName { name } if name.as_ref() == "not_a_parameter"
    ));
    let Err(retry) = method("combine").invoke(failure.into_invocation()) else {
        panic!("recovery retains unknown named bindings");
    };
    assert!(matches!(
        retry.error().kind(),
        InvocationErrorKind::UnknownArgumentName { name } if name.as_ref() == "not_a_parameter"
    ));

    let Err(failure) = method("combine").invoke(
        Invocation::<Local>::on_ref(&target)
            .named_arg("left", 1_i32)
            .named_arg("left", 2_i32),
    ) else {
        panic!("duplicate named bindings are rejected");
    };
    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::DuplicateArgumentBinding { name } if name.as_ref() == "left"
    ));
    let Err(retry) = method("combine").invoke(failure.into_invocation()) else {
        panic!("recovery retains duplicate named bindings");
    };
    assert!(matches!(
        retry.error().kind(),
        InvocationErrorKind::DuplicateArgumentBinding { name } if name.as_ref() == "left"
    ));

    let Err(failure) = method("combine").invoke(Invocation::<Local>::on_ref(&target).arg(1_i32)) else {
        panic!("missing parameters are rejected");
    };
    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::MissingArgument { index: 1 }
    ));
    let Err(retry) = method("combine").invoke(failure.into_invocation()) else {
        panic!("recovery retains the incomplete positional binding");
    };
    assert!(matches!(
        retry.error().kind(),
        InvocationErrorKind::MissingArgument { index: 1 }
    ));

    let Err(failure) = method("combine").invoke(Invocation::<Local>::on_ref(&target).arg(1_i32).arg(2_i32).arg(3_i32))
    else {
        panic!("excess positional bindings are rejected");
    };
    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::TooManyArguments { expected: 2, actual: 3 }
    ));
    let Err(retry) = method("combine").invoke(failure.into_invocation()) else {
        panic!("recovery retains all excess positional bindings");
    };
    assert!(matches!(
        retry.error().kind(),
        InvocationErrorKind::TooManyArguments { expected: 2, actual: 3 }
    ));
}

#[test]
fn invocation_type_mode_visibility_and_availability_failures_are_structured() {
    let target = InvocationTarget {
        _local_state: Rc::new(()),
        value: 4,
    };

    let Err(failure) = method("combine").invoke(Invocation::<Local>::on_ref(&target).arg_ref(&7_i32).arg(2_i32)) else {
        panic!("an owned parameter rejects a shared argument binding");
    };
    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::ArgumentModeMismatch { index: 0 }
    ));
    let Err(retry) = method("combine").invoke(failure.into_invocation()) else {
        panic!("argument modes remain unchanged after recovery");
    };
    assert!(matches!(
        retry.error().kind(),
        InvocationErrorKind::ArgumentModeMismatch { index: 0 }
    ));

    let other = OtherTarget;
    let Err(failure) = method("combine").invoke(Invocation::<Local>::on_ref(&other).arg(1_i32).arg(2_i32)) else {
        panic!("receiver type identity is checked before adapter dispatch");
    };
    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::ReceiverTypeMismatch { .. }
    ));

    let Err(failure) = method("private_call").invoke(Invocation::<Local>::on_ref(&target).arg(1_i32)) else {
        panic!("private method visibility is enforced by safe invocation");
    };
    assert!(matches!(failure.error().kind(), InvocationErrorKind::VisibilityDenied));
    let recovered = failure.into_invocation();
    let Ok(output) = (unsafe { method("private_call").invoke_unchecked(recovered) }) else {
        panic!("the recovered receiver and argument support explicit unchecked retry");
    };
    assert_eq!(
        output
            .into_owned::<i32>()
            .unwrap_or_else(|_| panic!("private method returns i32")),
        5
    );
}

#[test]
fn mutable_borrow_argument_mode_mismatch_does_not_consume_the_input() {
    let mut target = InvocationTarget {
        _local_state: Rc::new(()),
        value: 4,
    };
    let value = 3_i32;
    let Err(failure) = method("adjust").invoke(Invocation::<Local>::on_mut(&mut target).arg_ref(&value)) else {
        panic!("a mutable reference parameter requires a mutable caller binding");
    };
    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::ArgumentModeMismatch { index: 0 }
    ));
    let Err(retry) = method("adjust").invoke(failure.into_invocation()) else {
        panic!("the original shared borrow remains after failed preflight");
    };
    assert!(matches!(
        retry.error().kind(),
        InvocationErrorKind::ArgumentModeMismatch { index: 0 }
    ));
    drop(retry);
    assert_eq!(target.value, 4, "preflight failure must not enter the method body");
}
