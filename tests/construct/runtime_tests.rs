// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::NamedConstructionInput;
use qubit_reflect::Reflect;
use qubit_reflect::StructUpdateInput;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::construct::ConstructionError;

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
fn skipped_fields_are_absent_and_disable_dynamic_construction() {
    let descriptor = TypeDescriptor::of::<SkippedRecord>();
    let record = SkippedRecord {
        id: 4,
        ignored: NotReflected,
    };

    assert_eq!(descriptor.fields().len(), 1);
    assert!(descriptor.field("ignored").is_none());
    assert!(descriptor.struct_construction().is_none());
    assert_eq!(*descriptor.field("id").unwrap().get::<_, u32>(&record).unwrap(), 4);
}

#[test]
fn update_preserves_unmodified_fields_and_replaces_selected_fields() {
    let base = Profile {
        id: 3,
        name: String::from("before"),
    };
    let updated: Profile = TypeDescriptor::of::<Profile>()
        .update_struct(StructUpdateInput::new(
            base,
            NamedConstructionInput::new().field("name", String::from("after")),
        ))
        .expect("valid override updates the owned base");

    assert_eq!(updated.id, 3);
    assert_eq!(updated.name, "after");
}

#[test]
fn construction_rejects_unknown_names_and_wrong_input_shapes_with_recovery() {
    let descriptor = TypeDescriptor::of::<Profile>();
    let unknown = descriptor
        .construct_struct::<Profile>(NamedConstructionInput::new().field("other", 4_u32))
        .expect_err("unknown fields are rejected before construction");
    assert!(matches!(unknown.error(), ConstructionError::UnknownField { name } if name.as_ref() == "other"));
    assert_eq!(unknown.into_any_parts().1.len(), 1);

    let wrong_shape = descriptor
        .construct_tuple::<Profile>(qubit_reflect::TupleConstructionInput::new().value(4_u32))
        .expect_err("named structs reject positional construction");
    assert!(matches!(wrong_shape.error(), ConstructionError::WrongShape { .. }));
    assert_eq!(wrong_shape.into_any_parts().1.len(), 1);
}

#[test]
fn update_validation_recovers_base_and_original_overrides() {
    let failure = TypeDescriptor::of::<Profile>()
        .update_struct::<Profile>(StructUpdateInput::new(
            Profile {
                id: 3,
                name: String::from("before"),
            },
            NamedConstructionInput::new()
                .field("name", String::from("after"))
                .field("id", String::from("wrong type")),
        ))
        .expect_err("an override type mismatch is rejected before updating the base");
    assert!(matches!(failure.error(), ConstructionError::ValueTypeMismatch { .. }));

    let (_, recovered) = failure.into_any_parts();
    let [
        qubit_reflect::construct::RecoveredAnyValue::Base(base),
        qubit_reflect::construct::RecoveredAnyValue::Named {
            name: first_name,
            value: first_value,
        },
        qubit_reflect::construct::RecoveredAnyValue::Named {
            name: second_name,
            value: second_value,
        },
    ] = recovered.as_ref()
    else {
        panic!("update recovery must retain base then override");
    };
    assert_eq!(
        base.downcast_ref::<Profile>().map(|base| base.name.as_str()),
        Some("before")
    );
    assert_eq!(first_name.as_ref(), "name");
    assert_eq!(first_value.downcast_ref::<String>().map(String::as_str), Some("after"));
    assert_eq!(second_name.as_ref(), "id");
    assert_eq!(
        second_value.downcast_ref::<String>().map(String::as_str),
        Some("wrong type")
    );
}

#[test]
fn update_rejects_wrong_base_without_losing_overrides() {
    let failure = TypeDescriptor::of::<Profile>()
        .update_struct::<Profile>(StructUpdateInput::from_any(
            Box::new(String::from("wrong base")),
            NamedConstructionInput::new().field("name", String::from("replacement")),
        ))
        .expect_err("the base must be the exact reflected root type");
    assert!(matches!(failure.error(), ConstructionError::BaseTypeMismatch { .. }));

    let (_, recovered) = failure.into_any_parts();
    let [
        qubit_reflect::construct::RecoveredAnyValue::Base(base),
        qubit_reflect::construct::RecoveredAnyValue::Named { name, value },
    ] = recovered.as_ref()
    else {
        panic!("base mismatch must retain both the base and override");
    };
    assert_eq!(base.downcast_ref::<String>().map(String::as_str), Some("wrong base"));
    assert_eq!(name.as_ref(), "name");
    assert_eq!(value.downcast_ref::<String>().map(String::as_str), Some("replacement"));
}

#[test]
fn update_rejects_a_wrong_base_and_keeps_override_values() {
    let failure = TypeDescriptor::of::<Profile>()
        .update_struct::<Profile>(StructUpdateInput::from_any(
            Box::new(String::from("wrong base")),
            NamedConstructionInput::new().field("name", String::from("replacement")),
        ))
        .expect_err("the base must be the exact reflected root type");
    assert!(matches!(failure.error(), ConstructionError::BaseTypeMismatch { .. }));

    let (_, recovered) = failure.into_any_parts();
    let [
        qubit_reflect::construct::RecoveredAnyValue::Base(base),
        qubit_reflect::construct::RecoveredAnyValue::Named { name, value },
    ] = recovered.as_ref()
    else {
        panic!("base-type failure must retain both base and override");
    };
    assert_eq!(base.downcast_ref::<String>().map(String::as_str), Some("wrong base"));
    assert_eq!(name.as_ref(), "name");
    assert_eq!(value.downcast_ref::<String>().map(String::as_str), Some("replacement"));
}
