// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![cfg(feature = "derive")]

use qubit_reflect::ConstructionError;
use qubit_reflect::NamedConstructionInput;
use qubit_reflect::Reflect;
use qubit_reflect::StructUpdateInput;
use qubit_reflect::TupleConstructionInput;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::capability::CloneAdapter;
use qubit_reflect::capability::DefaultAdapter;
use qubit_reflect::descriptor::AssociatedConstReader;

#[Reflect]
#[derive(Debug)]
pub struct PublicRecord {
    pub id: u64,
    pub name: String,
}

#[Reflect]
#[derive(Debug)]
pub struct PublicTuple(pub u16, pub String);

#[Reflect]
#[derive(Debug)]
pub enum PublicEvent {
    Created { pub_id: u64 },
    Empty,
}

#[Reflect]
#[derive(Debug)]
pub struct RestrictedRecord {
    pub visible: u32,
    hidden: String,
}

#[test]
fn named_input_constructs_typed_or_dynamic_results() {
    let descriptor = TypeDescriptor::of::<PublicRecord>();
    let value: PublicRecord = descriptor
        .construct_struct(
            NamedConstructionInput::new()
                .field("id", 1_u64)
                .field("name", String::from("Ada")),
        )
        .expect("valid named construction");
    assert_eq!(value.id, 1);
    assert_eq!(value.name, "Ada");

    let erased = descriptor
        .construct_struct_any(
            NamedConstructionInput::new()
                .field("id", 2_u64)
                .field("name", String::from("Grace")),
        )
        .expect("valid dynamic construction");
    let value = erased.downcast::<PublicRecord>().expect("exact root type");
    assert_eq!(value.name, "Grace");
}

#[test]
fn tuple_and_variant_construction_accept_plain_values() {
    let tuple: PublicTuple = TypeDescriptor::of::<PublicTuple>()
        .construct_tuple(TupleConstructionInput::new().value(8_u16).value(String::from("eight")))
        .expect("valid tuple construction");
    assert_eq!(tuple.0, 8);

    let variant = TypeDescriptor::of::<PublicEvent>()
        .variant("Created")
        .expect("variant exists");
    let event: PublicEvent = variant
        .construct_struct(NamedConstructionInput::new().field("pub_id", 9_u64))
        .expect("valid variant construction");
    assert!(variant.is_active(&event).expect("exact enum type"));
}

#[test]
fn failures_recover_original_values_in_input_order() {
    let recovery = TypeDescriptor::of::<PublicRecord>()
        .construct_struct::<PublicRecord>(
            NamedConstructionInput::new()
                .field("name", String::from("Ada"))
                .field_any("id", Box::new(17_i32) as Box<dyn Any>),
        )
        .expect_err("wrong exact field type");
    assert!(matches!(recovery.error(), ConstructionError::ValueTypeMismatch { .. }));
    let (_, values) = recovery.into_any_parts();
    let [
        qubit_reflect::construct::RecoveredAnyValue::Named {
            name: first_name,
            value: first_value,
        },
        qubit_reflect::construct::RecoveredAnyValue::Named {
            name: second_name,
            value: second_value,
        },
    ] = values.as_ref()
    else {
        panic!("all values must be recovered in caller order");
    };
    assert_eq!(first_name.as_ref(), "name");
    assert_eq!(first_value.downcast_ref::<String>().map(String::as_str), Some("Ada"));
    assert_eq!(second_name.as_ref(), "id");
    assert_eq!(second_value.downcast_ref::<i32>(), Some(&17));
}

#[test]
fn shape_binding_and_missing_input_errors_keep_structured_categories() {
    let descriptor = TypeDescriptor::of::<PublicRecord>();
    let missing = descriptor
        .construct_struct::<PublicRecord>(NamedConstructionInput::new().field("id", 1_u64))
        .expect_err("required name is missing");
    assert!(matches!(missing.error(), ConstructionError::MissingField { .. }));
    assert_eq!(missing.into_any_parts().1.len(), 1);

    let duplicate = descriptor
        .construct_struct::<PublicRecord>(
            NamedConstructionInput::new()
                .field("id", 1_u64)
                .field("id", 2_u64)
                .field("name", String::from("Ada")),
        )
        .expect_err("duplicate field names are rejected");
    assert!(matches!(duplicate.error(), ConstructionError::DuplicateField { .. }));
    assert_eq!(duplicate.into_any_parts().1.len(), 3);

    let wrong_shape = descriptor
        .construct_tuple::<PublicRecord>(TupleConstructionInput::new())
        .expect_err("named structs reject positional construction");
    assert!(matches!(wrong_shape.error(), ConstructionError::WrongShape { .. }));
    assert!(wrong_shape.into_any_parts().1.is_empty());
}

#[test]
fn failed_update_recovers_the_base_first_then_ordered_overrides() {
    let failure = TypeDescriptor::of::<PublicRecord>()
        .update_struct::<PublicRecord>(StructUpdateInput::from_any(
            Box::new(String::from("wrong base")),
            NamedConstructionInput::new().field("name", String::from("replacement")),
        ))
        .expect_err("base root type is checked before updates");
    assert!(matches!(failure.error(), ConstructionError::BaseTypeMismatch { .. }));
    let (_, values) = failure.into_any_parts();
    let [
        qubit_reflect::construct::RecoveredAnyValue::Base(base),
        qubit_reflect::construct::RecoveredAnyValue::Named { name, value },
    ] = values.as_ref()
    else {
        panic!("base must be first, followed by original override order");
    };
    assert_eq!(base.downcast_ref::<String>().map(String::as_str), Some("wrong base"));
    assert_eq!(name.as_ref(), "name");
    assert_eq!(value.downcast_ref::<String>().map(String::as_str), Some("replacement"));
}

#[test]
fn ordinary_construction_and_update_reject_non_public_fields() {
    let descriptor = TypeDescriptor::of::<RestrictedRecord>();
    let input = NamedConstructionInput::new()
        .field("visible", 1_u32)
        .field("hidden", String::from("secret"));
    let failure = descriptor
        .construct_struct::<RestrictedRecord>(input)
        .expect_err("private field is restricted");
    assert!(matches!(failure.error(), ConstructionError::VisibilityDenied { .. }));

    // SAFETY: this test deliberately initializes the private field with its
    // required String invariant.
    let value: RestrictedRecord = unsafe {
        descriptor.construct_struct_unsafe(
            NamedConstructionInput::new()
                .field("visible", 1_u32)
                .field("hidden", String::from("secret")),
        )
    }
    .expect("explicit visibility bypass");
    assert_eq!(value.hidden, "secret");

    let update = StructUpdateInput::new(
        RestrictedRecord {
            visible: 2,
            hidden: String::from("before"),
        },
        NamedConstructionInput::new().field("visible", 3_u32),
    );
    assert!(matches!(
        descriptor
            .update_struct::<RestrictedRecord>(update)
            .expect_err("private fields are rebuilt")
            .error(),
        ConstructionError::VisibilityDenied { .. }
    ));

    // SAFETY: the replacement keeps the private String invariant and this test owns
    // the base value.
    let updated: RestrictedRecord = unsafe {
        descriptor.update_struct_unsafe(StructUpdateInput::new(
            RestrictedRecord {
                visible: 2,
                hidden: String::from("before"),
            },
            NamedConstructionInput::new().field("hidden", String::from("after")),
        ))
    }
    .expect("explicit visibility bypass");
    assert_eq!(updated.hidden, "after");
}

#[test]
fn updates_take_and_return_plain_values() {
    let updated: PublicRecord = TypeDescriptor::of::<PublicRecord>()
        .update_struct(StructUpdateInput::new(
            PublicRecord {
                id: 3,
                name: String::from("before"),
            },
            NamedConstructionInput::new().field("name", String::from("after")),
        ))
        .expect("valid update");
    assert_eq!(updated.name, "after");
}

#[test]
fn optional_projection_clone_default_and_associated_constants_use_plain_values() {
    let optional = TypeDescriptor::of::<Option<u32>>()
        .as_optional()
        .expect("optional type");
    assert_eq!(
        optional.project_ref(&Some(4_u32)).expect("exact optional type"),
        Some(&4)
    );
    assert_eq!(optional.project_ref(&None::<u32>).expect("empty optional"), None);

    let source = String::from("clone me");
    let cloned = CloneAdapter::new::<String>()
        .clone_value(&source)
        .expect("exact clone type");
    assert_eq!(cloned.downcast_ref::<String>().map(String::as_str), Some("clone me"));

    let defaulted = DefaultAdapter::new::<String>().create();
    assert_eq!(defaulted.downcast_ref::<String>().map(String::as_str), Some(""));

    fn constant() -> u8 {
        7
    }
    let reader = AssociatedConstReader::from_getter(constant);
    assert_eq!(reader.read::<u8>().expect("exact associated value type"), 7);
}

#[test]
fn boxed_dynamic_inputs_use_the_boxed_value_type() {
    let descriptor = TypeDescriptor::of::<PublicRecord>();
    let value: PublicRecord = descriptor
        .construct_struct(
            NamedConstructionInput::new()
                .field_any("id", Box::new(11_u64) as Box<dyn Any>)
                .field("name", String::from("boxed")),
        )
        .expect("field_any uses the box's concrete value type");
    assert_eq!(value.id, 11);

    let failure = descriptor
        .construct_struct::<PublicRecord>(
            NamedConstructionInput::new()
                .field("id", Box::new(12_u64) as Box<dyn Any>)
                .field("name", String::from("boxed")),
        )
        .expect_err("ordinary field treats Box<dyn Any> as the exact field value");
    assert!(matches!(failure.error(), ConstructionError::ValueTypeMismatch { .. }));
    let (_, values) = failure.into_any_parts();
    let qubit_reflect::construct::RecoveredAnyValue::Named { value, .. } = &values[0] else {
        panic!("first input retains its original name and order");
    };
    assert_eq!(
        value
            .downcast_ref::<Box<dyn Any>>()
            .and_then(|value| value.downcast_ref::<u64>()),
        Some(&12)
    );
}
use std::any::Any;
