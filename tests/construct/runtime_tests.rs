// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::Reflect;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::construct::ConstructionAvailability;
use qubit_reflect::construct::ConstructionError;
use qubit_reflect::construct::ConstructionInput;
use qubit_reflect::construct::ConstructionShape;
use qubit_reflect::construct::NamedFields;
use qubit_reflect::construct::PositionalFields;
use qubit_reflect::construct::RecoveredConstructionInput;
use qubit_reflect::construct::UnitFields;
use qubit_reflect::construct::UpdateInput;
use qubit_reflect::value::Local;

struct NotReflected;

#[Reflect]
pub struct SkippedRecord {
    pub id: u32,
    #[reflect(skip)]
    // This member remains a Rust field but must not enter reflection metadata.
    #[allow(dead_code)]
    ignored: NotReflected,
}

#[Reflect]
#[derive(Debug)]
pub struct Profile {
    pub id: u32,
    pub name: String,
}

#[test]
fn skipped_fields_are_absent_and_disable_construction_with_a_reason() {
    let descriptor = TypeDescriptor::of::<SkippedRecord>();
    let record = SkippedRecord {
        id: 4,
        ignored: NotReflected,
    };

    assert_eq!(descriptor.fields().len(), 1);
    assert!(descriptor.fields().get("ignored").is_none());
    assert!(matches!(
        descriptor.construction().availability(),
        ConstructionAvailability::Unavailable { .. }
    ));
    assert_eq!(*descriptor.field("id").unwrap().get::<_, u32>(&record).unwrap(), 4);
}

#[test]
fn update_preserves_unmodified_fields_and_replaces_selected_fields() {
    let updated: Profile = TypeDescriptor::of::<Profile>()
        .update(
            UpdateInput::<Local>::new(Profile {
                id: 3,
                name: String::from("before"),
            })
            .field("name", String::from("after")),
        )
        .expect("valid override updates the owned base");

    assert_eq!(updated.id, 3);
    assert_eq!(updated.name, "after");
}

#[test]
fn construction_rejects_unknown_names_and_wrong_input_shapes_with_recovery() {
    let descriptor = TypeDescriptor::of::<Profile>();
    let unknown = descriptor
        .construct::<Profile, _>(NamedFields::<Local>::new().field("other", 4_u32))
        .expect_err("unknown fields are rejected before construction");
    assert!(matches!(unknown.error(), ConstructionError::UnknownField { field } if field.as_ref() == "other"));
    assert!(matches!(unknown.into_input(), RecoveredConstructionInput::Named(_)));

    let wrong_shape = descriptor
        .construct::<Profile, _>(PositionalFields::<Local>::new().value(4_u32))
        .expect_err("named structs reject positional construction");
    assert!(matches!(
        wrong_shape.error(),
        ConstructionError::ShapeMismatch {
            expected: ConstructionShape::Named,
            actual: ConstructionShape::Positional,
        }
    ));
    assert!(matches!(
        wrong_shape.into_input(),
        RecoveredConstructionInput::Positional(_)
    ));
}

#[test]
fn update_validation_recovers_base_and_original_overrides() {
    let descriptor = TypeDescriptor::of::<Profile>();
    let failure = descriptor
        .update::<Profile, Local>(
            UpdateInput::<Local>::new(Profile {
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
            .downcast_ref::<Profile>()
            .map(|base| base.name.as_str()),
        Some("before")
    );

    let retry = descriptor
        .update::<Profile, Local>(failure.into_input())
        .expect_err("the recovered update retains its original invalid replacement");
    assert!(matches!(
        retry.error(),
        qubit_reflect::construct::UpdateError::ValueTypeMismatch { field: "id", .. }
    ));
}

#[test]
fn wrong_update_base_is_recovered_without_losing_overrides() {
    let descriptor = TypeDescriptor::of::<Profile>();
    let failure = descriptor
        .update::<Profile, Local>(
            UpdateInput::<Local>::new(String::from("wrong base")).field("name", String::from("replacement")),
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
        .update::<Profile, Local>(failure.into_input())
        .expect_err("retrying preserves the mismatched base and overrides");
    assert!(matches!(
        retry.error(),
        qubit_reflect::construct::UpdateError::BaseTypeMismatch(_)
    ));
}

#[test]
fn wrong_update_kind_is_structured() {
    let failure = TypeDescriptor::of::<Event>()
        .update::<Event, Local>(UpdateInput::<Local>::new(Event::Started))
        .expect_err("owned-value update only applies to structs");
    assert!(matches!(
        failure.error(),
        qubit_reflect::construct::UpdateError::WrongTargetKind
    ));
}

#[Reflect]
#[derive(Debug)]
enum Event {
    Started,
}

#[test]
fn unit_shapes_remain_explicit_in_the_public_input_model() {
    use qubit_reflect::value::Local;

    assert_eq!(
        <UnitFields as ConstructionInput<Local>>::shape(&UnitFields),
        ConstructionShape::Unit
    );
    assert_eq!(NamedFields::<Local>::new().shape(), ConstructionShape::Named);
    assert_eq!(PositionalFields::<Local>::new().shape(), ConstructionShape::Positional);
}
