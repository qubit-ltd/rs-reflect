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
use qubit_reflect::construct::ConstructionAvailability;
use qubit_reflect::construct::ConstructionFieldPolicy;
use qubit_reflect::construct::ConstructionShape;
use qubit_reflect::construct::ConstructionUnavailableReason;
use qubit_reflect::construct::UpdateFieldPolicy;
use qubit_reflect::construct::UpdateUnavailableReason;
use qubit_reflect::descriptor::Visibility;

#[Reflect]
struct InputRecord {
    pub value: u32,
}

#[Reflect]
struct InputTuple(pub u32);

#[Reflect]
struct PrivateInputRecord {
    pub label: String,
    secret: String,
}

#[Reflect]
struct PrivateInputTuple(pub String, u32);

#[Reflect]
struct EmptyInput;

#[test]
fn construction_descriptor_exposes_shape_and_field_policies() {
    let construction = TypeDescriptor::of::<InputRecord>().construction();
    assert_eq!(construction.shape(), ConstructionShape::Named);
    assert_eq!(construction.fields().len(), 1);
    let field = construction.fields().get(0).expect("declared field");
    assert_eq!(field.name(), "value");
    assert_eq!(field.policy(), ConstructionFieldPolicy::Required);
    assert_eq!(field.update_policy(), UpdateFieldPolicy::Replaceable);

    let tuple = TypeDescriptor::of::<InputTuple>().construction();
    assert_eq!(tuple.shape(), ConstructionShape::Positional);
    assert_eq!(tuple.fields().len(), 1);

    let unit = TypeDescriptor::of::<EmptyInput>().construction();
    assert_eq!(unit.shape(), ConstructionShape::Unit);
    assert!(unit.fields().is_empty());
}

#[test]
fn explicit_field_policy_metadata_is_typed_and_debuggable() {
    let required = qubit_reflect::construct::ConstructField::typed::<u32>(
        "value",
        Visibility::Public,
        ConstructionFieldPolicy::Required,
        UpdateFieldPolicy::Replaceable,
    );
    assert_eq!(required.name(), "value");
    assert_eq!(required.policy(), ConstructionFieldPolicy::Required);
    assert_eq!(required.update_policy(), UpdateFieldPolicy::Replaceable);

    let unavailable = qubit_reflect::construct::ConstructField::typed::<String>(
        "secret",
        Visibility::Private,
        ConstructionFieldPolicy::Unavailable(ConstructionUnavailableReason::PrivateRequiredField),
        UpdateFieldPolicy::Unavailable(UpdateUnavailableReason::FieldNotReplaceable),
    );
    assert_eq!(
        unavailable.policy(),
        ConstructionFieldPolicy::Unavailable(ConstructionUnavailableReason::PrivateRequiredField)
    );
    assert_eq!(
        unavailable.update_policy(),
        UpdateFieldPolicy::Unavailable(UpdateUnavailableReason::FieldNotReplaceable)
    );
}

#[test]
fn private_field_metadata_keeps_shape_and_reports_unavailability() {
    let construction = TypeDescriptor::of::<PrivateInputRecord>().construction();
    assert!(matches!(
        construction.availability(),
        ConstructionAvailability::Unavailable { reasons }
            if reasons.contains(&ConstructionUnavailableReason::PrivateRequiredField)
    ));
    assert_eq!(construction.fields().len(), 2);
    assert_eq!(construction.fields().get(1).unwrap().name(), "secret");

    let tuple = TypeDescriptor::of::<PrivateInputTuple>().construction();
    assert_eq!(tuple.shape(), ConstructionShape::Positional);
    assert_eq!(tuple.fields().len(), 2);
}
