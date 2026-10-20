// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![cfg(feature = "derive")]

use qubit_reflect::TypeDescriptor;
use qubit_reflect::construct::ConstructionAvailability;
use qubit_reflect::construct::NamedFields;
use qubit_reflect::construct::PositionalFields;
use qubit_reflect::construct::UnitFields;
use qubit_reflect::value::Local;

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
    assert!(descriptor.field("value").is_ok());
    assert!(descriptor.field("hidden").is_err());
    assert!(matches!(
        descriptor.construction().availability(),
        ConstructionAvailability::Unavailable { .. }
    ));
}

#[test]
fn named_tuple_and_unit_structs_construct_from_ordinary_values() {
    let named = TypeDescriptor::of::<Named>()
        .construct::<Named, _>(
            NamedFields::<Local>::new()
                .field("id", 7_u64)
                .field("label", String::from("Ada")),
        )
        .expect("named fields construct");
    assert_eq!(named.id, 7);
    assert_eq!(named.label, "Ada");

    let pair = TypeDescriptor::of::<Pair>()
        .construct::<Pair, _>(PositionalFields::<Local>::new().value(9_u8).value(String::from("nine")))
        .expect("tuple fields construct");
    assert_eq!(pair.0, 9);
    assert_eq!(pair.1, "nine");

    let unit = TypeDescriptor::of::<Unit>()
        .construct::<Unit, _>(UnitFields)
        .expect("unit struct constructs");
    let _ = unit;
}

#[test]
fn enum_variants_are_queryable_active_and_constructible() {
    let descriptor = TypeDescriptor::of::<Event>();
    assert_eq!(descriptor.variants().len(), 3);
    assert!(descriptor.variant("Absent").is_err());
    assert!(descriptor.variant("Hidden").is_err());

    let named_variant = descriptor.variant("Named").expect("named variant exists");
    assert_eq!(
        named_variant.construction().availability(),
        ConstructionAvailability::Available
    );
    let named: Event = named_variant
        .construct(NamedFields::<Local>::new().field("pub_value", String::from("ok")))
        .expect("public named variant construction");
    assert!(descriptor.variant("Named").unwrap().is_active(&named).unwrap());

    let tuple_variant = descriptor.variant("Tuple").expect("tuple variant exists");
    let tuple: Event = tuple_variant
        .construct(
            PositionalFields::<Local>::new()
                .value(16_u16)
                .value(String::from("sixteen")),
        )
        .expect("public tuple variant construction");
    assert!(descriptor.variant("Tuple").unwrap().is_active(&tuple).unwrap());

    let empty = descriptor
        .variant("Empty")
        .expect("unit variant exists")
        .construct::<Event, _>(UnitFields)
        .expect("unit variant constructs");
    assert!(descriptor.variant("Empty").unwrap().is_active(&empty).unwrap());
}

#[test]
fn cfg_and_cfg_attr_fields_follow_the_compiled_rust_shape() {
    let descriptor = TypeDescriptor::of::<ConditionalFields>();
    assert_eq!(descriptor.fields().len(), 1);
    assert!(descriptor.field("retained").is_ok());
    assert!(descriptor.field("absent_by_cfg").is_err());
    assert!(descriptor.field("absent_by_cfg_attr").is_err());
}

#[test]
fn skipped_variant_payload_field_keeps_type_reflection_but_disables_construction() {
    let variant = TypeDescriptor::of::<SkippedPayload>()
        .variant("Visible")
        .expect("variant remains reflected");
    assert_eq!(variant.fields().len(), 1);
    assert!(variant.field("internal").is_err());
    assert!(matches!(
        variant.construction().availability(),
        ConstructionAvailability::Unavailable { .. }
    ));

    let tuple = TypeDescriptor::of::<SkippedTuplePayload>()
        .variant("Values")
        .expect("tuple variant remains reflected");
    assert_eq!(tuple.fields().len(), 2);
    assert_eq!(tuple.fields().get_at(0).unwrap().index(), 0);
    assert_eq!(tuple.fields().get_at(1).unwrap().index(), 2);
    assert!(matches!(
        tuple.construction().availability(),
        ConstructionAvailability::Unavailable { .. }
    ));
}
