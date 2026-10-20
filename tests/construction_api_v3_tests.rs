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
use qubit_reflect::value::Local;

#[Reflect]
#[derive(Debug, Eq, PartialEq)]
struct PublicRecord {
    pub id: u64,
    pub name: String,
}

#[Reflect]
#[derive(Debug, Eq, PartialEq)]
struct PublicTuple(pub u16, pub String);

#[Reflect]
#[derive(Debug, Eq, PartialEq)]
pub enum PublicEvent {
    Created { pub_id: u64 },
    Empty,
}

#[Reflect]
#[derive(Debug)]
struct RestrictedRecord {
    pub visible: u32,
    hidden: String,
}

#[test]
fn named_positional_and_variant_construction_use_plain_values() {
    let value: PublicRecord = TypeDescriptor::of::<PublicRecord>()
        .construct(
            NamedFields::<Local>::new()
                .field("id", 1_u64)
                .field("name", String::from("Ada")),
        )
        .expect("valid named construction");
    assert_eq!(
        value,
        PublicRecord {
            id: 1,
            name: String::from("Ada")
        }
    );

    let tuple: PublicTuple = TypeDescriptor::of::<PublicTuple>()
        .construct(
            PositionalFields::<Local>::new()
                .value(8_u16)
                .value(String::from("eight")),
        )
        .expect("valid positional construction");
    assert_eq!(tuple, PublicTuple(8, String::from("eight")));

    let descriptor = TypeDescriptor::of::<PublicEvent>();
    let event: PublicEvent = descriptor
        .variant("Created")
        .expect("variant exists")
        .construct(NamedFields::<Local>::new().field("pub_id", 9_u64))
        .expect("valid variant construction");
    assert!(descriptor.variant("Created").unwrap().is_active(&event).unwrap());
}

#[test]
fn dynamic_construction_returns_the_exact_root_value() {
    let erased = TypeDescriptor::of::<PublicRecord>()
        .construct_dynamic::<_>(
            NamedFields::<Local>::new()
                .field("id", 2_u64)
                .field("name", String::from("Grace")),
        )
        .expect("valid dynamic construction");
    let value = erased.downcast::<PublicRecord>().expect("exact root type");
    assert_eq!(value.name, "Grace");
}

#[test]
fn failed_construction_preserves_the_original_input_for_retry() {
    let descriptor = TypeDescriptor::of::<PublicRecord>();
    let failure = descriptor
        .construct::<PublicRecord, _>(
            NamedFields::<Local>::new()
                .field("name", String::from("Ada"))
                .field("id", 17_i32),
        )
        .expect_err("wrong exact field type");
    assert!(matches!(
        failure.error(),
        ConstructionError::ValueTypeMismatch { field: "id", .. }
    ));

    let RecoveredConstructionInput::Named(input) = failure.into_input() else {
        panic!("named values remain recoverable as named input");
    };
    let retry = descriptor
        .construct::<PublicRecord, _>(input)
        .expect_err("retry retains the invalid field value");
    assert!(matches!(
        retry.error(),
        ConstructionError::ValueTypeMismatch { field: "id", .. }
    ));
}

#[test]
fn update_preserves_unmodified_values_and_recovers_invalid_bases() {
    let descriptor = TypeDescriptor::of::<PublicRecord>();
    let updated: PublicRecord = descriptor
        .update(
            UpdateInput::<Local>::new(PublicRecord {
                id: 3,
                name: String::from("before"),
            })
            .field("name", String::from("after")),
        )
        .expect("valid update");
    assert_eq!(updated.id, 3);
    assert_eq!(updated.name, "after");

    let failure = descriptor
        .update::<PublicRecord, Local>(
            UpdateInput::<Local>::new(String::from("wrong base")).field("name", String::from("replacement")),
        )
        .expect_err("the update base must have the reflected root type");
    assert!(matches!(
        failure.error(),
        qubit_reflect::construct::UpdateError::BaseTypeMismatch(_)
    ));
    assert_eq!(
        failure.recovery().base().downcast_ref::<String>().map(String::as_str),
        Some("wrong base")
    );
}

#[test]
fn private_fields_require_unchecked_construction_and_update() {
    let descriptor = TypeDescriptor::of::<RestrictedRecord>();
    let failure = descriptor
        .construct::<RestrictedRecord, _>(
            NamedFields::<Local>::new()
                .field("visible", 1_u32)
                .field("hidden", String::from("secret")),
        )
        .expect_err("safe construction cannot initialize private fields");
    assert!(matches!(failure.error(), ConstructionError::Unavailable(_)));

    // SAFETY: this test supplies a String satisfying the private field's invariant.
    let value: RestrictedRecord = unsafe {
        descriptor.construct_unchecked(
            NamedFields::<Local>::new()
                .field("visible", 1_u32)
                .field("hidden", String::from("secret")),
        )
    }
    .expect("explicit visibility bypass");
    assert_eq!(value.hidden, "secret");

    // SAFETY: the update replaces the private String with another valid String.
    let updated: RestrictedRecord =
        unsafe { descriptor.update_unchecked(UpdateInput::<Local>::new(value).field("hidden", String::from("after"))) }
            .expect("explicit update visibility bypass");
    assert_eq!(updated.hidden, "after");
}

#[test]
fn wrong_shape_errors_keep_the_structured_category_and_input() {
    let failure = TypeDescriptor::of::<PublicRecord>()
        .construct::<PublicRecord, _>(PositionalFields::<Local>::new())
        .expect_err("named structs reject positional input");
    assert!(matches!(failure.error(), ConstructionError::ShapeMismatch { .. }));
    assert!(matches!(
        failure.into_input(),
        RecoveredConstructionInput::Positional(_)
    ));

    let empty: PublicEvent = TypeDescriptor::of::<PublicEvent>()
        .variant("Empty")
        .unwrap()
        .construct(UnitFields::new())
        .expect("unit variant construction");
    assert_eq!(empty, PublicEvent::Empty);
}
