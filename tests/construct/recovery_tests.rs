// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::cell::Cell;
use std::rc::Rc;

use qubit_reflect::NamedConstructionInput;
use qubit_reflect::Reflect;
use qubit_reflect::StructUpdateInput;
use qubit_reflect::TupleConstructionInput;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::construct::ConstructionError;
use qubit_reflect::construct::RecoveredAnyValue;

#[Reflect]
#[derive(Debug)]
pub struct User {
    pub id: u64,
    pub name: String,
}

#[Reflect]
#[derive(Debug)]
pub struct Pair(pub u64, pub String);

struct DropProbe(Rc<Cell<usize>>);

impl Drop for DropProbe {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}

#[test]
fn validation_failure_recovers_original_named_values_in_caller_order() {
    let recovery = TypeDescriptor::of::<User>()
        .construct_struct::<User>(
            NamedConstructionInput::new()
                .field("name", String::from("Ada"))
                .field_any("id", Box::new(17_i32)),
        )
        .expect_err("field type mismatch must fail validation");

    assert!(matches!(recovery.error(), ConstructionError::ValueTypeMismatch { .. }));
    let (_, values) = recovery.into_any_parts();
    let [
        RecoveredAnyValue::Named {
            name: first_name,
            value: first_value,
        },
        RecoveredAnyValue::Named {
            name: second_name,
            value: second_value,
        },
    ] = values.as_ref()
    else {
        panic!("each supplied value must be retained");
    };
    assert_eq!(first_name.as_ref(), "name");
    assert_eq!(first_value.downcast_ref::<String>().map(String::as_str), Some("Ada"));
    assert_eq!(second_name.as_ref(), "id");
    assert_eq!(second_value.downcast_ref::<i32>(), Some(&17));
}

#[test]
fn missing_and_duplicate_inputs_report_structured_errors_and_recover_values() {
    let missing = TypeDescriptor::of::<User>()
        .construct_struct::<User>(NamedConstructionInput::new().field("id", 1_u64))
        .expect_err("required name is missing");
    assert!(matches!(missing.error(), ConstructionError::MissingField { .. }));
    assert_eq!(missing.into_any_parts().1.len(), 1);

    let duplicate = TypeDescriptor::of::<User>()
        .construct_struct::<User>(
            NamedConstructionInput::new()
                .field("id", 1_u64)
                .field("id", 2_u64)
                .field("name", String::from("Ada")),
        )
        .expect_err("duplicate names are rejected");
    assert!(matches!(duplicate.error(), ConstructionError::DuplicateField { .. }));
    assert_eq!(duplicate.into_any_parts().1.len(), 3);
}

#[test]
fn failed_validation_keeps_owned_input_until_recovery_is_dropped() {
    let drops = Rc::new(Cell::new(0));
    let recovery = TypeDescriptor::of::<User>()
        .construct_struct::<User>(
            NamedConstructionInput::new()
                .field("id", DropProbe(Rc::clone(&drops)))
                .field("name", String::from("Ada")),
        )
        .expect_err("the exact field type is checked before construction");

    assert!(matches!(recovery.error(), ConstructionError::ValueTypeMismatch { .. }));
    assert_eq!(drops.get(), 0, "validation must not drop the original input");
    let (_, values) = recovery.into_any_parts();
    let [RecoveredAnyValue::Named { value, .. }, RecoveredAnyValue::Named { .. }] = values.as_ref() else {
        panic!("recovery must preserve both original bindings");
    };
    assert!(value.downcast_ref::<DropProbe>().is_some());
    assert_eq!(drops.get(), 0, "inspecting recovery must retain ownership");
    drop(values);
    assert_eq!(drops.get(), 1, "the recovered value must be dropped exactly once");
}

#[test]
fn positional_construction_recovery_preserves_indices_and_values() {
    let recovery = TypeDescriptor::of::<Pair>()
        .construct_tuple::<Pair>(TupleConstructionInput::new().value(7_i32).value(String::from("seven")))
        .expect_err("the first tuple field has the wrong exact type");

    assert!(matches!(recovery.error(), ConstructionError::ValueTypeMismatch { .. }));
    assert!(format!("{recovery:?}").contains("ConstructionRecovery"));
    assert!(recovery.to_string().contains("construction value type mismatch"));
    let (_, values) = recovery.into_any_parts();
    let [
        RecoveredAnyValue::Positional {
            index: first_index,
            value: first_value,
        },
        RecoveredAnyValue::Positional {
            index: second_index,
            value: second_value,
        },
    ] = values.as_ref()
    else {
        panic!("all positional values must be recovered in caller order");
    };
    assert_eq!(*first_index, 0);
    assert_eq!(first_value.downcast_ref::<i32>(), Some(&7));
    assert_eq!(*second_index, 1);
    assert_eq!(second_value.downcast_ref::<String>().map(String::as_str), Some("seven"));
}

#[test]
fn failed_update_recovery_keeps_base_before_named_overrides() {
    let recovery = TypeDescriptor::of::<User>()
        .update_struct::<User>(StructUpdateInput::from_any(
            Box::new(String::from("wrong base")),
            NamedConstructionInput::new().field("name", String::from("replacement")),
        ))
        .expect_err("the base must have the exact reflected root type");

    assert!(matches!(recovery.error(), ConstructionError::BaseTypeMismatch { .. }));
    let (_, values) = recovery.into_any_parts();
    let [RecoveredAnyValue::Base(base), RecoveredAnyValue::Named { name, value }] = values.as_ref() else {
        panic!("the base must precede every override in recovery order");
    };
    assert_eq!(base.downcast_ref::<String>().map(String::as_str), Some("wrong base"));
    assert_eq!(name.as_ref(), "name");
    assert_eq!(value.downcast_ref::<String>().map(String::as_str), Some("replacement"));
}
