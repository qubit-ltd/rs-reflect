// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![cfg(feature = "derive")]

use qubit_reflect::Reflect;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::construct::ConstructionError;
use qubit_reflect::construct::NamedFields;
use qubit_reflect::construct::PositionalFields;
use qubit_reflect::construct::RecoveredConstructionInput;
use qubit_reflect::construct::UnitFields;
use qubit_reflect::construct::UpdateInput;
use qubit_reflect::descriptor::FieldAccessError;
use qubit_reflect::value::DynamicOwned;
use qubit_reflect::value::Local;

#[Reflect]
#[derive(Debug, PartialEq)]
struct NamedRecord {
    pub count: u32,
    secret: String,
}

#[Reflect]
#[derive(Debug, PartialEq)]
struct TupleRecord(pub u32, pub String);

#[Reflect]
#[derive(Debug, PartialEq)]
struct UnitRecord;

#[Reflect]
#[derive(Debug, PartialEq)]
struct PublicRecord {
    pub count: u32,
}

#[test]
fn unchecked_named_construction_bypasses_visibility_but_checks_types() {
    let descriptor = TypeDescriptor::of::<NamedRecord>();
    assert!(matches!(
        descriptor.construction().availability(),
        qubit_reflect::construct::ConstructionAvailability::Unavailable { .. }
    ));

    // SAFETY: both values have the declared field types; the private String
    // field is initialized with a valid String.
    let value = unsafe {
        descriptor.construct_unchecked::<NamedRecord, _>(
            NamedFields::<Local>::new()
                .field("count", 4_u32)
                .field("secret", String::from("private")),
        )
    }
    .expect("unchecked named construction succeeds");
    assert_eq!(
        value,
        NamedRecord {
            count: 4,
            secret: String::from("private"),
        }
    );

    // SAFETY: visibility is bypassed, while the intentionally wrong value
    // type is still rejected before the generated adapter runs.
    let failure = unsafe {
        descriptor.construct_unchecked::<NamedRecord, _>(
            NamedFields::<Local>::new()
                .field("count", String::from("wrong"))
                .field("secret", String::from("retained")),
        )
    }
    .expect_err("unchecked construction still checks exact field types");
    assert!(matches!(
        failure.error(),
        ConstructionError::ValueTypeMismatch { field: "count", .. }
    ));
    assert!(matches!(failure.into_input(), RecoveredConstructionInput::Named(_)));
}

#[test]
fn dynamic_unchecked_construction_and_typed_root_checks() {
    let descriptor = TypeDescriptor::of::<NamedRecord>();
    // SAFETY: both values match the declared types and preserve the private
    // field invariant.
    let erased = unsafe {
        descriptor.construct_dynamic_unchecked::<_>(
            NamedFields::<Local>::new()
                .field("count", 8_u32)
                .field("secret", String::from("dynamic")),
        )
    }
    .expect("dynamic unchecked construction succeeds");
    assert_eq!(
        erased.downcast::<NamedRecord>().expect("exact root type"),
        NamedRecord {
            count: 8,
            secret: String::from("dynamic"),
        }
    );

    // SAFETY: values match the private type's field types; the requested root
    // type intentionally does not match the descriptor.
    let failure = unsafe {
        descriptor.construct_unchecked::<TupleRecord, _>(
            NamedFields::<Local>::new()
                .field("count", 9_u32)
                .field("secret", String::from("wrong root")),
        )
    }
    .expect_err("unchecked construction does not bypass root type identity");
    assert!(matches!(failure.error(), ConstructionError::TargetTypeMismatch(_)));
}

#[test]
fn unchecked_positional_and_unit_construction_support_dynamic_results() {
    let tuple = TypeDescriptor::of::<TupleRecord>();
    // SAFETY: the supplied values match the positional field types.
    let typed: TupleRecord = unsafe {
        tuple.construct_unchecked(
            PositionalFields::<Local>::new()
                .value(7_u32)
                .value(String::from("seven")),
        )
    }
    .expect("typed unchecked tuple construction succeeds");
    assert_eq!(typed, TupleRecord(7, String::from("seven")));

    // SAFETY: the supplied values match the positional field types.
    let erased = unsafe {
        tuple.construct_dynamic_unchecked(
            PositionalFields::<Local>::new()
                .value(8_u32)
                .value(String::from("eight")),
        )
    }
    .expect("dynamic unchecked tuple construction succeeds");
    assert_eq!(
        erased.downcast::<TupleRecord>().expect("exact tuple root"),
        TupleRecord(8, String::from("eight"))
    );

    let unit = TypeDescriptor::of::<UnitRecord>();
    // SAFETY: a unit struct has no field invariants to violate.
    let typed: UnitRecord =
        unsafe { unit.construct_unchecked(UnitFields::new()) }.expect("typed unchecked unit construction");
    assert_eq!(typed, UnitRecord);

    // SAFETY: same empty unit construction contract as above.
    let erased =
        unsafe { unit.construct_dynamic_unchecked(UnitFields::new()) }.expect("dynamic unchecked unit construction");
    assert_eq!(erased.downcast::<UnitRecord>().expect("exact unit root"), UnitRecord);
}

#[test]
fn unchecked_update_bypasses_visibility_and_preserves_type_checks() {
    let descriptor = TypeDescriptor::of::<NamedRecord>();
    let base = NamedRecord {
        count: 5,
        secret: String::from("before"),
    };

    // SAFETY: the replacement is a String and preserves the private field's
    // invariant.
    let updated: NamedRecord =
        unsafe { descriptor.update_unchecked(UpdateInput::<Local>::new(base).field("secret", String::from("after"))) }
            .expect("unchecked update permits a private field replacement");
    assert_eq!(updated.secret, "after");

    // SAFETY: this intentionally supplies the wrong field value type to check
    // that bypassing visibility does not bypass exact type validation.
    let failure = unsafe {
        descriptor.update_unchecked::<NamedRecord, Local>(
            UpdateInput::<Local>::new(NamedRecord {
                count: 6,
                secret: String::from("before"),
            })
            .field("secret", 17_u32),
        )
    }
    .expect_err("unchecked update still checks exact replacement types");
    assert!(matches!(
        failure.error(),
        qubit_reflect::construct::UpdateError::ValueTypeMismatch { field: "secret", .. }
    ));
    assert_eq!(
        failure
            .recovery()
            .base()
            .downcast_ref::<NamedRecord>()
            .expect("the recovered update base keeps its original type")
            .count,
        6
    );
}

#[test]
fn unsafe_field_access_bypasses_visibility_but_checks_target_type() {
    let field = TypeDescriptor::of::<NamedRecord>()
        .field("secret")
        .expect("private field remains described");
    let mut record = NamedRecord {
        count: 1,
        secret: String::from("old"),
    };

    // SAFETY: the test owns the record and intentionally bypasses the field's
    // source visibility to mutate the String value.
    *unsafe { field.get_mut_unsafe::<NamedRecord, String>(&mut record) }
        .expect("unsafe mutable access bypasses visibility") = String::from("new");
    assert_eq!(record.secret, "new");

    let wrong_target = 9_u32;
    // SAFETY: this deliberately supplies the wrong root to verify target
    // identity remains checked even when visibility is bypassed.
    let error = unsafe { field.get_unsafe::<u32, String>(&wrong_target) }.expect_err("wrong dynamic root is rejected");
    assert!(matches!(error, FieldAccessError::TargetTypeMismatch(_)));
}

#[test]
fn dynamic_owned_inputs_keep_runtime_type_validation_and_recovery() {
    let public = TypeDescriptor::of::<PublicRecord>();
    let updated = public
        .update_dynamic(UpdateInput::<Local>::new(PublicRecord { count: 1 }).field("count", 2_u32))
        .expect("public dynamic update succeeds");
    assert_eq!(
        updated.downcast::<PublicRecord>().expect("exact updated root"),
        PublicRecord { count: 2 }
    );

    let update_failure = match public
        .update_dynamic(UpdateInput::<Local>::new(PublicRecord { count: 3 }).field("count", String::from("wrong")))
    {
        Ok(_) => panic!("dynamic update rejects the wrong replacement type"),
        Err(failure) => failure,
    };
    assert!(matches!(
        update_failure.error(),
        qubit_reflect::construct::UpdateError::ValueTypeMismatch { field: "count", .. }
    ));

    let descriptor = TypeDescriptor::of::<TupleRecord>();
    let failure = descriptor
        .construct::<TupleRecord, _>(
            PositionalFields::<Local>::new().value_dynamic(DynamicOwned::new(String::from("wrong"))),
        )
        .expect_err("dynamic input is checked against the positional field type");
    assert!(matches!(
        failure.error(),
        ConstructionError::ValueTypeMismatch { field: "0", .. }
    ));
    assert!(matches!(
        failure.into_input(),
        RecoveredConstructionInput::Positional(_)
    ));
}
