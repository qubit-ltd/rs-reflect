// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![cfg(feature = "derive")]

use qubit_reflect::TypeDescriptor;
use qubit_reflect::construct::NamedConstructionInput;
use qubit_reflect::construct::TupleConstructionInput;

pub struct NotReflect;

#[qubit_reflect::Reflect]
#[allow(dead_code)]
struct GenericRecord<T> {
    pub value: T,
    #[reflect(skip)]
    hidden: NotReflect,
}

#[qubit_reflect::Reflect]
struct Named {
    pub id: u64,
    pub label: String,
}

#[qubit_reflect::Reflect]
struct Pair(pub u8, pub String);

#[qubit_reflect::Reflect]
struct Unit;

#[qubit_reflect::Reflect]
struct ConditionalFields {
    pub retained: u8,
    #[cfg(any())]
    absent_by_cfg: NotReflect,
    #[cfg_attr(all(), cfg(any()))]
    absent_by_cfg_attr: NotReflect,
}

#[qubit_reflect::Reflect]
pub enum Event {
    Named {
        pub_value: String,
    },
    Tuple(u16, String),
    Empty,
    #[cfg(any())]
    Absent,
    #[reflect(skip)]
    Hidden(NotReflect),
}

#[qubit_reflect::Reflect]
#[allow(dead_code)]
enum SkippedPayload {
    Visible {
        value: u8,
        #[reflect(skip)]
        internal: NotReflect,
    },
}

#[qubit_reflect::Reflect]
#[allow(dead_code)]
enum SkippedTuplePayload {
    Values(u8, #[reflect(skip)] NotReflect, String),
}

#[test]
fn structs_exclude_skipped_fields_without_reflect_or_default_bounds() {
    let descriptor = TypeDescriptor::of::<GenericRecord<u32>>();
    assert_eq!(descriptor.fields().len(), 1);
    assert!(descriptor.field("value").is_some());
    assert!(descriptor.field("hidden").is_none());
    assert!(descriptor.struct_construction().is_none());
}

#[test]
fn named_tuple_and_unit_structs_construct_from_ordinary_values() {
    let named = TypeDescriptor::of::<Named>()
        .construct_struct::<Named>(
            NamedConstructionInput::new()
                .field("id", 7_u64)
                .field("label", String::from("Ada")),
        )
        .expect("named fields construct");
    assert_eq!(named.id, 7);
    assert_eq!(named.label, "Ada");

    let pair = TypeDescriptor::of::<Pair>()
        .construct_tuple::<Pair>(TupleConstructionInput::new().value(9_u8).value(String::from("nine")))
        .expect("tuple fields construct");
    assert_eq!(pair.0, 9);
    assert_eq!(pair.1, "nine");

    let unit = TypeDescriptor::of::<Unit>()
        .construct_unit::<Unit>()
        .expect("unit struct constructs");
    let _ = unit;
}

#[test]
fn enum_variants_are_queryable_active_and_constructible() {
    let descriptor = TypeDescriptor::of::<Event>();
    assert_eq!(descriptor.variants().len(), 3);
    assert!(descriptor.variant("Absent").is_none());
    assert!(descriptor.variant("Hidden").is_none());

    let named = descriptor
        .variant("Named")
        .expect("named variant exists")
        .construct_struct::<Event>(NamedConstructionInput::new().field("pub_value", String::from("ok")))
        .expect("named variant constructs");
    assert!(descriptor.variant("Named").unwrap().is_active(&named).unwrap());

    let tuple = descriptor
        .variant("Tuple")
        .expect("tuple variant exists")
        .construct_tuple::<Event>(
            TupleConstructionInput::new()
                .value(16_u16)
                .value(String::from("sixteen")),
        )
        .expect("tuple variant constructs");
    assert!(descriptor.variant("Tuple").unwrap().is_active(&tuple).unwrap());

    let empty = descriptor
        .variant("Empty")
        .expect("unit variant exists")
        .construct_unit::<Event>()
        .expect("unit variant constructs");
    assert!(descriptor.variant("Empty").unwrap().is_active(&empty).unwrap());
}

#[test]
fn cfg_and_cfg_attr_fields_follow_the_compiled_rust_shape() {
    let descriptor = TypeDescriptor::of::<ConditionalFields>();
    assert_eq!(descriptor.fields().len(), 1);
    assert!(descriptor.field("retained").is_some());
    assert!(descriptor.field("absent_by_cfg").is_none());
    assert!(descriptor.field("absent_by_cfg_attr").is_none());
}

#[test]
fn skipped_variant_payload_field_keeps_type_reflection_but_disables_construction() {
    let variant = TypeDescriptor::of::<SkippedPayload>()
        .variant("Visible")
        .expect("variant remains reflected");
    assert_eq!(variant.fields().len(), 1);
    assert!(variant.field("internal").is_none());
    assert!(variant.construction().is_none());

    let tuple = TypeDescriptor::of::<SkippedTuplePayload>()
        .variant("Values")
        .expect("tuple variant remains reflected");
    assert_eq!(tuple.fields().len(), 2);
    assert_eq!(tuple.fields()[0].index(), 0);
    assert_eq!(tuple.fields()[1].index(), 2);
    assert!(tuple.field_at(1).is_none());
    assert!(tuple.construction().is_none());
}
