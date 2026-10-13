// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![cfg(feature = "derive")]

use std::any::Any;

use qubit_reflect::NamedConstructionInput;
use qubit_reflect::Reflect;
use qubit_reflect::StructUpdateInput;
use qubit_reflect::TupleConstructionInput;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::access::FieldAccessError;
use qubit_reflect::construct::ConstructionError;
use qubit_reflect::construct::RecoveredAnyValue;
use qubit_reflect::value::DynamicMut;
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
fn unsafe_dynamic_construction_checks_types_and_returns_exact_values() {
    let named = TypeDescriptor::of::<NamedRecord>();
    // SAFETY: both supplied values have the declared field types.
    let value = unsafe {
        named.construct_struct_any_unsafe(
            NamedConstructionInput::new()
                .field("count", 4_u32)
                .field("secret", String::from("private")),
        )
    }
    .expect("unsafe dynamic named construction succeeds");
    let value = value.downcast::<NamedRecord>().expect("exact named root type");
    assert_eq!(
        *value,
        NamedRecord {
            count: 4,
            secret: String::from("private"),
        }
    );

    // SAFETY: visibility is bypassed, but the intentionally wrong value type
    // must still be rejected before the generated adapter runs.
    let recovery = unsafe {
        named.construct_struct_any_unsafe(
            NamedConstructionInput::new()
                .field("count", String::from("wrong"))
                .field("secret", String::from("retained")),
        )
    }
    .expect_err("unsafe construction still validates exact field types");
    let (error, values) = recovery.into_any_parts();
    assert!(matches!(error, ConstructionError::ValueTypeMismatch { .. }));
    let values = values.into_vec();
    let [
        RecoveredAnyValue::Named { name, value },
        RecoveredAnyValue::Named {
            name: secret,
            value: secret_value,
        },
    ] = values.as_slice()
    else {
        panic!("recovery preserves each named input in caller order");
    };
    assert_eq!(name.as_ref(), "count");
    assert_eq!(value.downcast_ref::<String>().map(String::as_str), Some("wrong"));
    assert_eq!(secret.as_ref(), "secret");
    assert_eq!(
        secret_value.downcast_ref::<String>().map(String::as_str),
        Some("retained")
    );
}

#[test]
fn unsafe_tuple_and_unit_construction_support_typed_and_erased_results() {
    let tuple = TypeDescriptor::of::<TupleRecord>();
    // SAFETY: the supplied values have the declared positional field types.
    let typed: TupleRecord = unsafe {
        tuple.construct_tuple_unsafe(TupleConstructionInput::new().value(7_u32).value(String::from("seven")))
    }
    .expect("typed unsafe tuple construction succeeds");
    assert_eq!(typed, TupleRecord(7, String::from("seven")));

    // SAFETY: the supplied values have the declared positional field types.
    let erased = unsafe {
        tuple.construct_tuple_any_unsafe(TupleConstructionInput::new().value(8_u32).value(String::from("eight")))
    }
    .expect("erased unsafe tuple construction succeeds");
    assert_eq!(
        *erased.downcast::<TupleRecord>().expect("exact tuple root"),
        TupleRecord(8, "eight".into())
    );

    let unit = TypeDescriptor::of::<UnitRecord>();
    // SAFETY: a unit struct has no private fields whose invariants could be
    // violated, and this call deliberately covers the public unsafe wrapper.
    let typed: UnitRecord = unsafe { unit.construct_unit_unsafe() }.expect("typed unsafe unit construction");
    assert_eq!(typed, UnitRecord);

    // SAFETY: same empty unit construction contract as above.
    let erased = unsafe { unit.construct_unit_any_unsafe() }.expect("erased unsafe unit construction");
    assert_eq!(*erased.downcast::<UnitRecord>().expect("exact unit root"), UnitRecord);

    // SAFETY: the method still rejects a typed result that differs from the
    // descriptor's exact root.
    let failure = unsafe { unit.construct_unit_unsafe::<TupleRecord>() }
        .expect_err("unsafe visibility bypass does not bypass target type identity");
    assert!(matches!(failure.error(), ConstructionError::TargetTypeMismatch { .. }));
}

#[test]
fn dynamic_update_returns_values_and_preserves_failed_inputs() {
    let public = TypeDescriptor::of::<PublicRecord>();
    let updated = public
        .update_struct_any(StructUpdateInput::new(
            PublicRecord { count: 1 },
            NamedConstructionInput::new().field("count", 2_u32),
        ))
        .expect("public dynamic update succeeds")
        .downcast::<PublicRecord>()
        .expect("exact updated root");
    assert_eq!(*updated, PublicRecord { count: 2 });

    let failure = public
        .update_struct_any(StructUpdateInput::new(
            PublicRecord { count: 3 },
            NamedConstructionInput::new().field("count", String::from("wrong")),
        ))
        .expect_err("dynamic update rejects the wrong replacement type");
    let (error, values) = failure.into_any_parts();
    assert!(matches!(error, ConstructionError::ValueTypeMismatch { .. }));
    let values = values.into_vec();
    let [RecoveredAnyValue::Base(base), RecoveredAnyValue::Named { name, value }] = values.as_slice() else {
        panic!("recovery retains the base followed by the rejected override");
    };
    assert_eq!(base.downcast_ref::<PublicRecord>().map(|base| base.count), Some(3));
    assert_eq!(name.as_ref(), "count");
    assert_eq!(value.downcast_ref::<String>().map(String::as_str), Some("wrong"));

    let private = TypeDescriptor::of::<NamedRecord>();
    let safe_failure = private
        .update_struct_any(StructUpdateInput::new(
            NamedRecord {
                count: 4,
                secret: String::from("before"),
            },
            NamedConstructionInput::new().field("secret", String::from("after")),
        ))
        .expect_err("safe dynamic update rejects a private field");
    assert!(matches!(
        safe_failure.error(),
        ConstructionError::VisibilityDenied { .. }
    ));
    let (_, values) = safe_failure.into_any_parts();
    assert!(matches!(values.first(), Some(RecoveredAnyValue::Base(_))));

    // SAFETY: this test intentionally replaces the private String with another
    // String and therefore preserves the type's private-field invariant.
    let updated = unsafe {
        private.update_struct_any_unsafe(StructUpdateInput::new(
            NamedRecord {
                count: 5,
                secret: String::from("before"),
            },
            NamedConstructionInput::new().field("secret", String::from("after")),
        ))
    }
    .expect("unsafe dynamic update permits the private field replacement")
    .downcast::<NamedRecord>()
    .expect("exact updated private root");
    assert_eq!(updated.secret, "after");
}

#[test]
fn unsafe_dynamic_mutation_preserves_visibility_and_target_checks() {
    let field = TypeDescriptor::of::<NamedRecord>()
        .field("secret")
        .expect("private field remains described");
    let mut record = NamedRecord {
        count: 1,
        secret: String::from("old"),
    };

    // SAFETY: the test owns the record and intentionally bypasses the field's
    // source visibility to mutate its String value.
    let mut secret = unsafe { field.get_mut_dynamic_unsafe(DynamicMut::<Local>::new(&mut record)) }
        .expect("unsafe dynamic mutable access bypasses visibility");
    *secret.downcast_mut::<String>().expect("exact field type") = String::from("new");
    assert_eq!(record.secret, "new");

    let mut wrong_target = 9_u32;
    // SAFETY: visibility bypass does not relax the target root type check.
    let error = match unsafe { field.get_mut_dynamic_unsafe(DynamicMut::<Local>::new(&mut wrong_target)) } {
        Ok(_) => panic!("wrong dynamic root is rejected"),
        Err(error) => error,
    };
    assert!(matches!(error, FieldAccessError::TargetTypeMismatch { .. }));
}

#[test]
fn tuple_value_any_checks_the_boxed_value_type_and_recovers_it_unchanged() {
    let descriptor = TypeDescriptor::of::<TupleRecord>();
    let input = TupleConstructionInput::new()
        .value_any(Box::new(11_u32) as Box<dyn Any>)
        .value_any(Box::new(String::from("eleven")) as Box<dyn Any>);
    let value: TupleRecord = descriptor
        .construct_tuple(input)
        .expect("boxed inputs use their inner exact types");
    assert_eq!(value, TupleRecord(11, String::from("eleven")));

    let failure = descriptor
        .construct_tuple::<TupleRecord>(TupleConstructionInput::new().value_any(Box::new(String::from("wrong"))))
        .expect_err("boxed Any input is checked against the first tuple field");
    let (error, values) = failure.into_any_parts();
    assert!(matches!(error, ConstructionError::ValueTypeMismatch { .. }));
    let values = values.into_vec();
    let [RecoveredAnyValue::Positional { index: 0, value }] = values.as_slice() else {
        panic!("the rejected boxed value keeps its original position");
    };
    assert_eq!(value.downcast_ref::<String>().map(String::as_str), Some("wrong"));
}
