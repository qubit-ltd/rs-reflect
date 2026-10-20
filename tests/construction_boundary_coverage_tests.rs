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
use qubit_reflect::construct::ConstructionShape;
use qubit_reflect::construct::NamedFields;
use qubit_reflect::construct::PositionalFields;
use qubit_reflect::construct::UnitFields;
use qubit_reflect::construct::UpdateInput;
use qubit_reflect::value::DynamicOwned;
use qubit_reflect::value::Local;
use qubit_reflect::value::ThreadSafe;

#[Reflect]
#[derive(Debug, Eq, PartialEq)]
struct ConstructionRecord {
    pub count: u32,
    pub label: String,
}

#[Reflect]
#[derive(Debug, Eq, PartialEq)]
struct ConstructionPair(pub u8, pub String);

#[Reflect]
#[derive(Debug, Eq, PartialEq)]
struct ConstructionUnit;

#[Reflect]
#[derive(Debug, Eq, PartialEq)]
enum ConstructionEvent {
    Ready,
}

#[test]
fn descriptor_target_and_field_iteration_report_declared_shape() {
    let descriptor = TypeDescriptor::of::<ConstructionRecord>().construction();
    assert!(matches!(
        descriptor.target(),
        qubit_reflect::construct::ConstructionTarget::Struct(_)
    ));
    assert_eq!(descriptor.shape(), ConstructionShape::Named);
    assert_eq!(
        descriptor.fields().iter().map(|field| field.name()).collect::<Vec<_>>(),
        ["count", "label"]
    );

    let variant = TypeDescriptor::of::<ConstructionEvent>()
        .variant("Ready")
        .expect("unit variant is registered")
        .construction();
    assert!(matches!(
        variant.target(),
        qubit_reflect::construct::ConstructionTarget::Variant(_)
    ));
    assert_eq!(variant.shape(), ConstructionShape::Unit);
    assert!(variant.fields().is_empty());
}

#[test]
fn builders_and_dynamic_construction_accept_local_and_thread_safe_values() {
    let local = TypeDescriptor::of::<ConstructionRecord>()
        .construct_dynamic::<_>(
            NamedFields::<Local>::new()
                .field_dynamic("count", DynamicOwned::new(3_u32))
                .field("label", String::from("local")),
        )
        .expect("local dynamic input constructs the reflected record")
        .downcast::<ConstructionRecord>()
        .expect("constructed value has the requested concrete type");
    assert_eq!(
        local,
        ConstructionRecord {
            count: 3,
            label: "local".into()
        }
    );

    let thread_safe = TypeDescriptor::of::<ConstructionRecord>()
        .construct::<ConstructionRecord, _>(
            NamedFields::<ThreadSafe>::default()
                .field("count", 5_u32)
                .field("label", String::from("shared")),
        )
        .expect("thread-safe input constructs the reflected record");
    assert_eq!(thread_safe.label, "shared");

    let pair = TypeDescriptor::of::<ConstructionPair>()
        .construct::<ConstructionPair, _>(
            PositionalFields::<Local>::default()
                .value(7_u8)
                .value_dynamic(DynamicOwned::new(String::from("seven"))),
        )
        .expect("positional builders accept typed and dynamic values");
    assert_eq!(pair, ConstructionPair(7, "seven".into()));

    let unit = TypeDescriptor::of::<ConstructionUnit>()
        .construct::<ConstructionUnit, _>(UnitFields::new())
        .expect("unit input constructs a unit struct");
    assert_eq!(unit, ConstructionUnit);
}

#[test]
fn thread_safe_dynamic_updates_and_public_errors_keep_their_contract() {
    let descriptor = TypeDescriptor::of::<ConstructionRecord>();
    let updated = descriptor
        .update_dynamic(
            UpdateInput::<ThreadSafe>::new(ConstructionRecord {
                count: 1,
                label: String::from("before"),
            })
            .field_dynamic("count", DynamicOwned::new(9_u32)),
        )
        .expect("thread-safe dynamic update accepts a matching field")
        .downcast::<ConstructionRecord>()
        .expect("updated dynamic value retains its concrete type");
    assert_eq!(updated.count, 9);

    // SAFETY: only a public u32 field is replaced with a value of the same type.
    let unchecked = unsafe {
        descriptor.update_dynamic_unchecked(
            UpdateInput::<Local>::new(ConstructionRecord {
                count: 2,
                label: String::from("safe"),
            })
            .field("count", 4_u32),
        )
    }
    .expect("unchecked dynamic update is valid for this public field");
    assert_eq!(unchecked.downcast_ref::<ConstructionRecord>().unwrap().count, 4);

    let construction_error = descriptor
        .construct::<ConstructionRecord, _>(NamedFields::<Local>::new())
        .expect_err("required fields cannot be omitted");
    assert!(matches!(
        construction_error.error(),
        qubit_reflect::construct::ConstructionError::MissingField { field: "count" }
    ));
    assert!(format!("{}", construction_error.error()).contains("MissingField"));
    assert_eq!(construction_error.recovery().shape(), ConstructionShape::Named);
    let recovered = construction_error.into_input();
    assert!(matches!(
        recovered,
        qubit_reflect::construct::RecoveredConstructionInput::Named(_)
    ));

    let update_error = descriptor
        .update::<ConstructionRecord, Local>(
            UpdateInput::new(ConstructionRecord {
                count: 1,
                label: String::new(),
            })
            .field("unknown", 2_u32),
        )
        .expect_err("unknown replacement fields are rejected");
    assert!(matches!(
        update_error.error(),
        qubit_reflect::construct::UpdateError::UnknownField { field } if field.as_ref() == "unknown"
    ));
    assert!(format!("{}", update_error.error()).contains("UnknownField"));
    assert!(format!("{update_error:?}").contains("UpdateFailure"));
    assert_eq!(
        update_error
            .recovery()
            .base()
            .downcast_ref::<ConstructionRecord>()
            .unwrap()
            .count,
        1
    );
    let _recovered_update = update_error.into_input();
}
