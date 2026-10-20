// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::cell::Cell;
use std::rc::Rc;

use qubit_reflect::Reflect;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::construct::ConstructionError;
use qubit_reflect::construct::NamedFields;
use qubit_reflect::construct::PositionalFields;
use qubit_reflect::construct::RecoveredConstructionInput;
use qubit_reflect::construct::UpdateInput;
use qubit_reflect::error::RecoverableFailure;
use qubit_reflect::value::Local;

#[Reflect]
#[derive(Debug)]
pub struct User {
    pub id: u64,
    pub name: String,
}

#[Reflect]
#[derive(Debug)]
pub struct Pair(pub u64, pub String);

#[Reflect]
struct DropProbe(Rc<Cell<usize>>);

impl Drop for DropProbe {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}

#[test]
fn validation_failure_recovers_the_original_named_input_for_retry() {
    let descriptor = TypeDescriptor::of::<User>();
    let failure = descriptor
        .construct::<User, _>(
            NamedFields::<Local>::new()
                .field("name", String::from("Ada"))
                .field("id", 17_i32),
        )
        .expect_err("field type mismatch must fail validation");
    assert!(matches!(
        failure.error(),
        ConstructionError::ValueTypeMismatch { field: "id", .. }
    ));

    let RecoveredConstructionInput::Named(input) = failure.into_input() else {
        panic!("the failed named input must be recoverable as named fields");
    };
    let retry = descriptor
        .construct::<User, _>(input)
        .expect_err("retrying the retained input reports the same type mismatch");
    assert!(matches!(
        retry.error(),
        ConstructionError::ValueTypeMismatch { field: "id", .. }
    ));
}

#[test]
fn construction_failure_exposes_owned_recovery_through_shared_trait() {
    let descriptor = TypeDescriptor::of::<User>();
    let failure = descriptor
        .construct::<User, _>(
            NamedFields::<Local>::new()
                .field("name", String::from("Ada"))
                .field("id", 17_i32),
        )
        .expect_err("field type mismatch must fail validation");

    assert!(matches!(
        RecoverableFailure::error(&failure),
        ConstructionError::ValueTypeMismatch { field: "id", .. }
    ));
    assert!(matches!(
        RecoverableFailure::recovery(&failure),
        RecoveredConstructionInput::Named(_)
    ));

    let (error, recovery) = RecoverableFailure::into_parts(failure);
    assert!(matches!(
        error,
        ConstructionError::ValueTypeMismatch { field: "id", .. }
    ));
    let RecoveredConstructionInput::Named(input) = recovery else {
        panic!("shared recovery protocol must retain the named construction input");
    };
    let retry = descriptor
        .construct::<User, _>(input)
        .expect_err("the owned trait recovery must be reusable as construction input");
    assert!(matches!(
        retry.error(),
        ConstructionError::ValueTypeMismatch { field: "id", .. }
    ));
}

#[test]
fn missing_and_duplicate_inputs_report_structured_errors_and_keep_their_shape() {
    let descriptor = TypeDescriptor::of::<User>();
    let missing = descriptor
        .construct::<User, _>(NamedFields::<Local>::new().field("id", 1_u64))
        .expect_err("required name is missing");
    assert!(matches!(
        missing.error(),
        ConstructionError::MissingField { field: "name" }
    ));
    assert!(matches!(missing.into_input(), RecoveredConstructionInput::Named(_)));

    let duplicate = descriptor
        .construct::<User, _>(
            NamedFields::<Local>::new()
                .field("id", 1_u64)
                .field("id", 2_u64)
                .field("name", String::from("Ada")),
        )
        .expect_err("duplicate names are rejected");
    assert!(matches!(
        duplicate.error(),
        ConstructionError::DuplicateField { field: "id" }
    ));
    let RecoveredConstructionInput::Named(input) = duplicate.into_input() else {
        panic!("duplicate named values must remain a named input");
    };
    let retry = descriptor
        .construct::<User, _>(input)
        .expect_err("the recovered duplicate bindings remain intact");
    assert!(matches!(
        retry.error(),
        ConstructionError::DuplicateField { field: "id" }
    ));
}

#[test]
fn rejected_owned_values_remain_owned_until_recovery_is_dropped() {
    let drops = Rc::new(Cell::new(0));
    let failure = TypeDescriptor::of::<User>()
        .construct::<User, _>(
            NamedFields::<Local>::new()
                .field("id", DropProbe(Rc::clone(&drops)))
                .field("name", String::from("Ada")),
        )
        .expect_err("the exact field type is checked before construction");

    assert!(matches!(
        failure.error(),
        ConstructionError::ValueTypeMismatch { field: "id", .. }
    ));
    assert_eq!(drops.get(), 0, "validation must retain the original input");

    let retry = match failure.into_input() {
        RecoveredConstructionInput::Named(input) => TypeDescriptor::of::<User>()
            .construct::<User, _>(input)
            .expect_err("the recovered value still has its original type"),
        _ => panic!("named values must recover as named fields"),
    };
    assert_eq!(drops.get(), 0, "the failed retry must still own the value");
    drop(retry);
    assert_eq!(drops.get(), 1, "the retained value is dropped exactly once");
}

#[test]
fn positional_recovery_retains_shape_and_original_type_error() {
    let descriptor = TypeDescriptor::of::<Pair>();
    let failure = descriptor
        .construct::<Pair, _>(
            PositionalFields::<Local>::new()
                .value(7_i32)
                .value(String::from("seven")),
        )
        .expect_err("the first tuple field has the wrong exact type");
    assert!(matches!(
        failure.error(),
        ConstructionError::ValueTypeMismatch { field: "0", .. }
    ));

    let RecoveredConstructionInput::Positional(input) = failure.into_input() else {
        panic!("positional values must recover as positional input");
    };
    let retry = descriptor
        .construct::<Pair, _>(input)
        .expect_err("retrying the recovered input reports the same mismatch");
    assert!(matches!(
        retry.error(),
        ConstructionError::ValueTypeMismatch { field: "0", .. }
    ));
}

#[test]
fn failed_update_retains_base_and_replacements_for_retry() {
    let descriptor = TypeDescriptor::of::<User>();
    let failure = descriptor
        .update::<User, Local>(
            UpdateInput::<Local>::new(User {
                id: 3,
                name: String::from("before"),
            })
            .field("name", String::from("after"))
            .field("id", String::from("wrong type")),
        )
        .expect_err("an override type mismatch is rejected before updating the base");
    assert!(matches!(
        failure.error(),
        qubit_reflect::construct::UpdateError::ValueTypeMismatch { field: "id", .. }
    ));
    assert_eq!(
        failure
            .recovery()
            .base()
            .downcast_ref::<User>()
            .map(|base| base.name.as_str()),
        Some("before"),
        "the original owned base remains available through recovery"
    );

    let retry = descriptor
        .update::<User, Local>(failure.into_input())
        .expect_err("retrying the recovered update preserves the invalid replacement");
    assert!(matches!(
        retry.error(),
        qubit_reflect::construct::UpdateError::ValueTypeMismatch { field: "id", .. }
    ));
}

#[test]
fn update_failure_exposes_owned_recovery_through_shared_trait() {
    let descriptor = TypeDescriptor::of::<User>();
    let failure = descriptor
        .update::<User, Local>(
            UpdateInput::<Local>::new(User {
                id: 3,
                name: String::from("before"),
            })
            .field("id", String::from("wrong type")),
        )
        .expect_err("replacement type mismatch must fail validation");

    assert!(matches!(
        RecoverableFailure::error(&failure),
        qubit_reflect::construct::UpdateError::ValueTypeMismatch { field: "id", .. }
    ));
    let _: &UpdateInput<Local> = RecoverableFailure::recovery(&failure);
    assert!(failure.base().downcast_ref::<User>().is_some());

    let (error, input) = RecoverableFailure::into_parts(failure);
    assert!(matches!(
        error,
        qubit_reflect::construct::UpdateError::ValueTypeMismatch { field: "id", .. }
    ));
    let retry = descriptor
        .update::<User, Local>(input)
        .expect_err("the owned trait recovery must be reusable as update input");
    assert!(matches!(
        retry.error(),
        qubit_reflect::construct::UpdateError::ValueTypeMismatch { field: "id", .. }
    ));
}

#[test]
fn wrong_update_base_keeps_base_and_replacements() {
    let descriptor = TypeDescriptor::of::<User>();
    let failure = descriptor
        .update::<User, Local>(
            UpdateInput::<Local>::from_dynamic(qubit_reflect::value::DynamicOwned::new(String::from("wrong base")))
                .field("name", String::from("replacement")),
        )
        .expect_err("the base must have the exact reflected root type");
    assert!(matches!(
        failure.error(),
        qubit_reflect::construct::UpdateError::BaseTypeMismatch(_)
    ));
    assert_eq!(
        failure.recovery().base().downcast_ref::<String>().map(String::as_str),
        Some("wrong base")
    );

    let retry = descriptor
        .update::<User, Local>(failure.into_input())
        .expect_err("recovered base type remains unchanged");
    assert!(matches!(
        retry.error(),
        qubit_reflect::construct::UpdateError::BaseTypeMismatch(_)
    ));
}
