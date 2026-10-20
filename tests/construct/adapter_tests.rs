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
use qubit_reflect::construct::ConstructionShape;
use qubit_reflect::construct::ConstructionUnavailableReason;
use qubit_reflect::construct::NamedFields;
use qubit_reflect::construct::PositionalFields;
use qubit_reflect::construct::UnitFields;
use qubit_reflect::construct::UpdateInput;
use qubit_reflect::value::Local;

#[Reflect]
#[derive(Debug, Eq, PartialEq)]
pub struct Profile {
    pub id: u32,
    pub label: String,
}

#[Reflect]
#[derive(Debug, Eq, PartialEq)]
pub struct Pair(pub u32, pub String);

#[Reflect]
#[derive(Debug, Eq, PartialEq)]
pub enum Event {
    Started,
    Failed { pub_code: u32, pub_message: String },
    Pair(u32, String),
}

#[Reflect]
#[derive(Debug)]
pub struct Restricted {
    pub id: u32,
    secret: String,
}

#[Reflect]
#[derive(Debug)]
pub struct OptionalValue {
    pub value: Option<u32>,
}

#[Reflect]
#[derive(Debug)]
pub struct RequiredValue {
    pub value: String,
}

#[Reflect]
#[derive(Debug)]
pub struct Unit;

#[test]
fn named_tuple_and_unit_structs_construct_from_plain_values() {
    let profile: Profile = TypeDescriptor::of::<Profile>()
        .construct(
            NamedFields::<Local>::new()
                .field("id", 7_u32)
                .field("label", String::from("Ada")),
        )
        .expect("named construction accepts ordinary values");
    assert_eq!(
        profile,
        Profile {
            id: 7,
            label: String::from("Ada")
        }
    );

    let pair: Pair = TypeDescriptor::of::<Pair>()
        .construct(
            PositionalFields::<Local>::new()
                .value(3_u32)
                .value(String::from("three")),
        )
        .expect("positional construction accepts ordinary values");
    assert_eq!(pair, Pair(3, String::from("three")));

    let unit: Unit = TypeDescriptor::of::<Unit>()
        .construct(UnitFields)
        .expect("unit construction accepts empty input");
    let _ = unit;
}

#[test]
fn construction_descriptors_report_shape_and_fields() {
    let named = TypeDescriptor::of::<Profile>().construction();
    assert_eq!(named.shape(), ConstructionShape::Named);
    assert!(matches!(named.availability(), ConstructionAvailability::Available));
    assert_eq!(named.fields().len(), 2);

    let tuple = TypeDescriptor::of::<Pair>().construction();
    assert_eq!(tuple.shape(), ConstructionShape::Positional);
    assert_eq!(tuple.fields().len(), 2);

    let unit = TypeDescriptor::of::<Unit>().construction();
    assert_eq!(unit.shape(), ConstructionShape::Unit);
    assert!(unit.fields().is_empty());
}

#[test]
fn enum_variants_construct_from_explicit_variant_descriptors() {
    let descriptor = TypeDescriptor::of::<Event>();
    let started: Event = descriptor
        .variant("Started")
        .expect("unit variant exists")
        .construct(UnitFields)
        .expect("unit variant construction");
    assert!(matches!(started, Event::Started));

    let failed_variant = descriptor.variant("Failed").expect("named variant exists");
    assert!(matches!(
        failed_variant.construction().availability(),
        ConstructionAvailability::Available
    ));
    let failed: Event = failed_variant
        .construct(
            NamedFields::<Local>::new()
                .field("pub_code", 500_u32)
                .field("pub_message", String::from("failure")),
        )
        .expect("public enum variant construction");
    assert!(matches!(failed, Event::Failed { pub_code: 500, .. }));

    let pair_variant = descriptor.variant("Pair").expect("tuple variant exists");
    assert!(matches!(
        pair_variant.construction().availability(),
        ConstructionAvailability::Available
    ));
    let pair: Event = pair_variant
        .construct(
            PositionalFields::<Local>::new()
                .value(8_u32)
                .value(String::from("eight")),
        )
        .expect("public tuple variant construction");
    assert!(matches!(pair, Event::Pair(8, _)));
}

#[test]
fn enum_construction_descriptor_tracks_variant_shape() {
    let variant = TypeDescriptor::of::<Event>()
        .variant("Pair")
        .expect("tuple variant exists");
    let construction = variant.construction();
    assert_eq!(construction.shape(), ConstructionShape::Positional);
    assert_eq!(construction.fields().len(), 2);
}

#[test]
fn required_fields_are_not_defaulted_and_unreflectable_options_are_unavailable() {
    let failure = TypeDescriptor::of::<RequiredValue>()
        .construct::<RequiredValue, _>(NamedFields::<Local>::new())
        .expect_err("required fields are not implicitly defaulted");
    assert!(matches!(
        failure.error(),
        ConstructionError::MissingField { field: "value" }
    ));
    assert!(matches!(
        failure.into_input(),
        qubit_reflect::construct::RecoveredConstructionInput::Named(_)
    ));

    let optional = TypeDescriptor::of::<OptionalValue>();
    assert!(matches!(
        optional.construction().availability(),
        ConstructionAvailability::Unavailable { reasons }
            if reasons.contains(&ConstructionUnavailableReason::FieldNotReflectable)
    ));
    let failure = optional
        .construct::<OptionalValue, _>(NamedFields::<Local>::new())
        .expect_err("an unreflectable field makes construction unavailable");
    assert!(matches!(
        failure.error(),
        ConstructionError::Unavailable(ConstructionAvailability::Unavailable { reasons })
            if reasons.contains(&ConstructionUnavailableReason::FieldNotReflectable)
    ));
    assert!(matches!(
        failure.into_input(),
        qubit_reflect::construct::RecoveredConstructionInput::Named(_)
    ));
}

#[test]
fn private_fields_require_unsafe_construction_and_update() {
    let descriptor = TypeDescriptor::of::<Restricted>();
    assert!(matches!(
        descriptor.construction().availability(),
        ConstructionAvailability::Unavailable { reasons }
            if reasons.contains(&ConstructionUnavailableReason::PrivateRequiredField)
    ));
    let failure = descriptor
        .construct::<Restricted, _>(
            NamedFields::<Local>::new()
                .field("id", 1_u32)
                .field("secret", String::from("hidden")),
        )
        .expect_err("safe construction cannot initialize private fields");
    assert!(matches!(
        failure.error(),
        ConstructionError::Unavailable(ConstructionAvailability::Unavailable { reasons })
            if reasons.contains(&ConstructionUnavailableReason::PrivateRequiredField)
    ));

    // SAFETY: the input initializes both fields with their declared types.
    let value: Restricted = unsafe {
        descriptor.construct_unchecked(
            NamedFields::<Local>::new()
                .field("id", 1_u32)
                .field("secret", String::from("hidden")),
        )
    }
    .expect("explicit visibility bypass");
    assert_eq!(value.secret, "hidden");

    // SAFETY: this test owns the value and replaces the private field with a
    // String.
    let updated: Restricted = unsafe {
        descriptor.update_unchecked(UpdateInput::<Local>::new(value).field("secret", String::from("changed")))
    }
    .expect("explicit update visibility bypass");
    assert_eq!(updated.secret, "changed");
}

#[test]
fn typed_and_dynamic_updates_preserve_unmodified_fields() {
    let descriptor = TypeDescriptor::of::<Profile>();
    let updated: Profile = descriptor
        .update(
            UpdateInput::<Local>::new(Profile {
                id: 5,
                label: String::from("before"),
            })
            .field("label", String::from("after")),
        )
        .expect("valid typed update");
    assert_eq!(updated.id, 5);
    assert_eq!(updated.label, "after");

    let updated = descriptor
        .update_dynamic(
            UpdateInput::<Local>::new(Profile {
                id: 8,
                label: String::from("dynamic before"),
            })
            .field("label", String::from("dynamic after")),
        )
        .expect("valid dynamic update")
        .downcast::<Profile>()
        .expect("dynamic update retains the reflected root type");
    assert_eq!(updated.id, 8);
    assert_eq!(updated.label, "dynamic after");
}

#[test]
fn construction_rejects_wrong_shapes_with_structured_recovery() {
    let profile = TypeDescriptor::of::<Profile>();
    let tuple_failure = profile
        .construct::<Profile, _>(PositionalFields::<Local>::new().value(7_u32))
        .expect_err("named struct cannot consume positional input");
    assert!(matches!(
        tuple_failure.error(),
        ConstructionError::ShapeMismatch {
            expected: ConstructionShape::Named,
            actual: ConstructionShape::Positional,
        }
    ));
    assert!(matches!(
        tuple_failure.into_input(),
        qubit_reflect::construct::RecoveredConstructionInput::Positional(_)
    ));

    let unit = TypeDescriptor::of::<Unit>();
    let named_failure = unit
        .construct::<Unit, _>(NamedFields::<Local>::new().field("extra", 1_u32))
        .expect_err("unit struct cannot consume named input");
    assert!(matches!(
        named_failure.error(),
        ConstructionError::ShapeMismatch {
            expected: ConstructionShape::Unit,
            actual: ConstructionShape::Named,
        }
    ));
    assert!(matches!(
        named_failure.into_input(),
        qubit_reflect::construct::RecoveredConstructionInput::Named(_)
    ));
}

#[test]
fn variant_construction_rejects_wrong_shape_and_checks_activity() {
    let unit_variant = TypeDescriptor::of::<Event>()
        .variant("Started")
        .expect("unit variant exists");
    let failure = unit_variant
        .construct::<Event, _>(NamedFields::<Local>::new().field("extra", 9_u32))
        .expect_err("unit variant rejects named construction input");
    assert!(
        matches!(
            failure.error(),
            ConstructionError::ShapeMismatch {
                expected: ConstructionShape::Unit,
                actual: ConstructionShape::Named,
            }
        ),
        "actual construction error: {:?}",
        failure.error()
    );
    assert!(matches!(
        failure.into_input(),
        qubit_reflect::construct::RecoveredConstructionInput::Named(_)
    ));

    let variant = TypeDescriptor::of::<Event>()
        .variant("Pair")
        .expect("tuple variant exists");
    assert!(
        variant
            .is_active(&Event::Pair(1, String::from("active")))
            .expect("the value has the declaring enum type")
    );
    assert!(
        !variant
            .is_active(&Event::Started)
            .expect("the sibling variant has the same declaring enum type")
    );
    assert!(variant.is_active(&17_u32).is_err());
}
