// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![cfg(feature = "derive")]

use std::any::Any;

use qubit_reflect::Reflect;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::construct::ConstructionField;
use qubit_reflect::construct::ConstructionFieldPolicy;
use qubit_reflect::construct::ConstructionUnavailableReason;
use qubit_reflect::construct::NamedConstructionInput;
use qubit_reflect::construct::StructUpdateInput;
use qubit_reflect::construct::TupleConstructionInput;
use qubit_reflect::value::DynamicOwned;
use qubit_reflect::value::Local;

#[Reflect]
#[derive(Debug)]
struct InputRecord {
    pub value: u32,
}

#[Reflect]
struct InputTuple(pub u32);

#[Reflect]
#[derive(Debug)]
struct PrivateInputRecord {
    pub label: String,
    secret: String,
}

#[Reflect]
#[derive(Debug)]
struct PrivateInputTuple(pub String, u32);

#[Reflect]
#[derive(Debug)]
struct EmptyInput;

fn provide_default() -> DynamicOwned<Local> {
    DynamicOwned::<Local>::new(99_u32)
}

#[test]
fn public_local_input_defaults_and_boxed_any_builders_work() {
    let named = NamedConstructionInput::default().field_any("value", Box::new(31_u32) as Box<dyn Any>);
    let value: InputRecord = TypeDescriptor::of::<InputRecord>()
        .construct_struct(named)
        .expect("field_any preserves the boxed value's concrete type");
    assert_eq!(value.value, 31);

    let tuple = TupleConstructionInput::default().value_any(Box::new(47_u32) as Box<dyn Any>);
    let value: InputTuple = TypeDescriptor::of::<InputTuple>()
        .construct_tuple(tuple)
        .expect("value_any preserves the boxed value's concrete type");
    assert_eq!(value.0, 47);

    let empty_named = NamedConstructionInput::default();
    let empty_tuple = TupleConstructionInput::default();
    assert!(format!("{empty_named:?}").contains("names: []"));
    assert!(format!("{empty_tuple:?}").contains("value_count: 0"));
}

#[test]
fn construction_field_policies_are_cloneable_and_debuggable() {
    let field = TypeDescriptor::of::<InputRecord>()
        .field("value")
        .expect("reflected field");

    let required = ConstructionField::<Local>::required(field);
    assert!(matches!(required.clone().policy(), ConstructionFieldPolicy::Required));
    assert_eq!(format!("{:?}", required.policy()), "Required");

    let defaulted = ConstructionField::<Local>::defaulted(field, provide_default);
    assert!(matches!(
        defaulted.clone().policy(),
        ConstructionFieldPolicy::Default(_)
    ));
    assert_eq!(format!("{:?}", defaulted.policy()), "Default(<provider>)");

    let provider_only = ConstructionField::<Local>::provider_only(field, provide_default);
    assert!(matches!(
        provider_only.clone().policy(),
        ConstructionFieldPolicy::ProviderOnly(_)
    ));
    assert_eq!(format!("{:?}", provider_only.policy()), "ProviderOnly(<provider>)");

    let unavailable = ConstructionField::<Local>::unavailable(field, ConstructionUnavailableReason::UpdateForbidden);
    assert!(matches!(
        unavailable.clone().policy(),
        ConstructionFieldPolicy::Unavailable(ConstructionUnavailableReason::UpdateForbidden)
    ));
    assert_eq!(format!("{:?}", unavailable.policy()), "Unavailable(UpdateForbidden)");
    assert!(format!("{defaulted:?}").contains("policy: Default(<provider>)"));
}

#[test]
fn updater_exposes_its_descriptor_and_declared_fields() {
    let descriptor = TypeDescriptor::of::<InputRecord>();
    let updater = descriptor
        .struct_construction()
        .expect("construction descriptor")
        .local_updater()
        .expect("generated updater");

    assert!(std::ptr::eq(updater.descriptor(), descriptor));
    assert_eq!(updater.fields().len(), 1);
    assert!(std::ptr::eq(
        updater.fields()[0].descriptor(),
        descriptor.field("value").unwrap()
    ));
    assert_eq!(format!("{:?}", updater.fields()[0].policy()), "Allowed");
}

#[test]
fn struct_constructor_exposes_metadata_shape_and_debug() {
    let descriptor = TypeDescriptor::of::<InputRecord>();
    let constructor = descriptor
        .struct_construction()
        .expect("construction descriptor")
        .local_constructor();

    assert!(std::ptr::eq(constructor.descriptor(), descriptor));
    assert_eq!(constructor.fields().len(), 1);
    assert!(std::ptr::eq(
        constructor.fields()[0].descriptor(),
        descriptor.field("value").unwrap()
    ));
    assert_eq!(constructor.shape(), qubit_reflect::construct::ConstructionShape::Named);
    let debug = format!("{constructor:?}");
    assert!(debug.contains("StructConstructor"));
    assert!(debug.contains("shape: Named"));
    assert!(debug.contains("query_name: Some(\"value\")"));
}

#[test]
fn unsafe_tuple_and_unit_constructors_cover_private_shapes() {
    let tuple_descriptor = TypeDescriptor::of::<PrivateInputTuple>();
    let tuple_constructor = tuple_descriptor
        .struct_construction()
        .expect("tuple construction descriptor")
        .local_constructor();
    assert_eq!(
        tuple_constructor.shape(),
        qubit_reflect::construct::ConstructionShape::Tuple
    );

    // SAFETY: the second private tuple field accepts any u32, preserving its
    // invariant.
    let tuple = unsafe {
        tuple_constructor.construct_tuple_unsafe(
            TupleConstructionInput::new()
                .value(String::from("visible"))
                .value(23_u32),
        )
    }
    .expect("explicit private tuple construction");
    let tuple = tuple
        .downcast::<PrivateInputTuple>()
        .unwrap_or_else(|_| panic!("constructor returned the wrong tuple root"));
    assert_eq!(tuple.0, "visible");
    assert_eq!(tuple.1, 23);

    let unit_descriptor = TypeDescriptor::of::<EmptyInput>();
    let unit_constructor = unit_descriptor
        .struct_construction()
        .expect("unit construction descriptor")
        .local_constructor();
    assert_eq!(
        unit_constructor.shape(),
        qubit_reflect::construct::ConstructionShape::Unit
    );
    // SAFETY: EmptyInput has no fields or private invariants.
    let unit = unsafe { unit_constructor.construct_unit_unsafe() }.expect("unit construction");
    assert!(unit.downcast_ref::<EmptyInput>().is_some());
}

#[test]
fn unsafe_named_construction_accepts_private_fields_and_recovers_bad_values() {
    let descriptor = TypeDescriptor::of::<PrivateInputRecord>();
    // SAFETY: the test provides a non-empty secret satisfying the private field
    // invariant.
    let value: PrivateInputRecord = unsafe {
        descriptor.construct_struct_unsafe(
            NamedConstructionInput::new()
                .field("label", String::from("public"))
                .field("secret", String::from("valid secret")),
        )
    }
    .expect("explicit private-field construction");
    assert_eq!(value.label, "public");
    assert_eq!(value.secret, "valid secret");

    // SAFETY: this request is expected to fail type validation before invoking the
    // adapter.
    let failure = unsafe {
        descriptor.construct_struct_unsafe::<PrivateInputRecord>(
            NamedConstructionInput::new()
                .field("label", String::from("public"))
                .field("secret", 12_u32),
        )
    }
    .expect_err("wrong private field type is rejected");
    assert!(matches!(
        failure.error(),
        qubit_reflect::ConstructionError::ValueTypeMismatch { .. }
    ));
    let (_, recovered) = failure.into_any_parts();
    assert_eq!(recovered.len(), 2);
    let [
        qubit_reflect::construct::RecoveredAnyValue::Named { value: label, .. },
        qubit_reflect::construct::RecoveredAnyValue::Named { value: secret, .. },
    ] = recovered.as_ref()
    else {
        panic!("all original named values must be recoverable");
    };
    assert_eq!(label.downcast_ref::<String>().map(String::as_str), Some("public"));
    assert_eq!(secret.downcast_ref::<u32>(), Some(&12));
}

#[test]
fn recovery_debug_formats_base_and_named_entries_without_erased_values() {
    let base_failure = TypeDescriptor::of::<InputRecord>()
        .update_struct::<InputRecord>(StructUpdateInput::new(
            String::from("wrong base"),
            NamedConstructionInput::new().field("value", 1_u32),
        ))
        .expect_err("wrong update base is rejected");
    let base_debug = format!("{base_failure:?}");
    assert!(base_debug.contains("Base(<value>)"));
    assert!(base_debug.contains("name: \"value\""));
    assert!(base_debug.contains("value: \"<value>\""));

    let named_failure = TypeDescriptor::of::<InputRecord>()
        .construct_struct::<InputRecord>(NamedConstructionInput::new().field("unknown", 8_u32))
        .expect_err("unknown field is rejected");
    let named_debug = format!("{named_failure:?}");
    assert!(named_debug.contains("name: \"unknown\""));
    assert!(named_debug.contains("value: \"<value>\""));
}

#[test]
fn duplicate_update_binding_recovers_each_original_value() {
    let failure = TypeDescriptor::of::<InputRecord>()
        .update_struct::<InputRecord>(StructUpdateInput::new(
            InputRecord { value: 1 },
            NamedConstructionInput::new()
                .field("value", 2_u32)
                .field("value", 3_u32),
        ))
        .expect_err("duplicate update bindings are rejected");

    let (error, recovered) = failure.into_any_parts();
    assert!(matches!(error, qubit_reflect::ConstructionError::DuplicateField { .. }));
    assert_eq!(recovered.len(), 3, "base and both duplicate values are retained");
    let [
        qubit_reflect::construct::RecoveredAnyValue::Base(base),
        qubit_reflect::construct::RecoveredAnyValue::Named {
            name: first_name,
            value: first,
        },
        qubit_reflect::construct::RecoveredAnyValue::Named {
            name: second_name,
            value: second,
        },
    ] = recovered.as_ref()
    else {
        panic!("update recovery preserves base-first input order");
    };
    assert_eq!(base.downcast_ref::<InputRecord>().map(|value| value.value), Some(1));
    assert_eq!(first_name.as_ref(), "value");
    assert_eq!(first.downcast_ref::<u32>(), Some(&2));
    assert_eq!(second_name.as_ref(), "value");
    assert_eq!(second.downcast_ref::<u32>(), Some(&3));
}
