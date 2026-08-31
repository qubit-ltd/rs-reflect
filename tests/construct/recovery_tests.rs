// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Integration tests for construction input and recovery inspection APIs.

use qubit_reflect::ConstructionError;
use qubit_reflect::NamedConstructionInput;
use qubit_reflect::ReflectedOwned;
use qubit_reflect::TupleConstructionInput;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::construct::RecoveredConstructionValue;
use qubit_reflect::construct::StructUpdateInput;

/// Verifies named input retains caller order and exposes recovery through all
/// public inspection and consuming APIs.
#[test]
fn test_named_construction_recovery_preserves_names_values_and_error() {
    let input = NamedConstructionInput::new([
        ("first", ReflectedOwned::new(7_u8)),
        ("second", ReflectedOwned::new(String::from("value"))),
    ]);
    assert_eq!(input.fields().len(), 2);
    assert_eq!(input.fields()[0].0.as_ref(), "first");
    assert!(format!("{input:?}").contains("first"));

    let Err(recovery) = TypeDescriptor::of::<u8>().construct_struct(input) else {
        panic!("primitive targets cannot construct named structs");
    };
    assert_eq!(recovery.error(), &ConstructionError::TargetUnavailable);
    assert_eq!(recovery.values().len(), 2);
    assert!(format!("{recovery:?}").contains("TargetUnavailable"));
    assert_eq!(
        recovery.to_string(),
        "dynamic construction is unavailable for this target"
    );

    let (error, values) = recovery.into_parts();
    assert_eq!(error, ConstructionError::TargetUnavailable);
    let RecoveredConstructionValue::Named { name, value } = &values[0] else {
        panic!("first recovery entry should be named");
    };
    assert_eq!(name.as_ref(), "first");
    assert_eq!(value.downcast_ref::<u8>(), Some(&7));
}

/// Verifies tuple and unit target-unavailable paths preserve positional values
/// and empty recovery respectively.
#[test]
fn test_tuple_and_unit_construction_recovery_preserve_shape() {
    let input = TupleConstructionInput::new([ReflectedOwned::new(11_u8), ReflectedOwned::new(12_u8)]);
    assert_eq!(input.values().len(), 2);
    assert!(format!("{input:?}").contains("value_count"));

    let Err(recovery) = TypeDescriptor::of::<u8>().construct_tuple(input) else {
        panic!("primitive targets cannot construct tuple structs");
    };
    let values = recovery.into_values();
    let RecoveredConstructionValue::Positional { index, value } = &values[1] else {
        panic!("second recovery entry should be positional");
    };
    assert_eq!(*index, 1);
    assert_eq!(value.downcast_ref::<u8>(), Some(&12));
    assert!(format!("{:?}", values[0]).contains("Positional"));

    let Err(unit_recovery) = TypeDescriptor::of::<u8>().construct_unit() else {
        panic!("primitive targets cannot construct unit structs");
    };
    assert!(unit_recovery.values().is_empty());
}

/// Verifies update input exposes an untouched base and ordered overrides
/// before validation.
#[test]
fn test_struct_update_input_exposes_base_and_overrides() {
    let input = StructUpdateInput::new(
        ReflectedOwned::new(3_u8),
        NamedConstructionInput::new([("value", ReflectedOwned::new(4_u8))]),
    );

    assert_eq!(input.base().downcast_ref::<u8>(), Some(&3));
    assert_eq!(input.overrides().fields()[0].0.as_ref(), "value");
    assert!(format!("{input:?}").contains("override_count"));
}
