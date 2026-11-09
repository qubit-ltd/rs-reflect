// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![cfg(feature = "derive")]

use std::rc::Rc;

use qubit_reflect::Reflect;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::construct::NamedFields;
use qubit_reflect::construct::UpdateInput;
use qubit_reflect::value::Local;
use qubit_reflect::value::ThreadSafe;

#[Reflect(local_only)]
struct LocalRecord {
    pub value: Rc<u8>,
}

#[Reflect(local_only)]
pub enum LocalEvent {
    Value(Rc<u8>),
}

#[test]
fn test_local_only_struct_supports_local_field_and_construction_operations() {
    let descriptor = TypeDescriptor::of::<LocalRecord>();
    let field = descriptor.field("value").unwrap();
    let mut value = LocalRecord { value: Rc::new(1_u8) };
    assert_eq!(**field.get::<_, Rc<u8>>(&value).unwrap(), 1);
    field.set(&mut value, Rc::new(2_u8)).unwrap();
    let made: LocalRecord = descriptor
        .construct(NamedFields::<Local>::new().field("value", Rc::new(3_u8)))
        .unwrap();
    assert_eq!(*made.value, 3);
    let updated = descriptor
        .update::<LocalRecord, _>(UpdateInput::<Local>::new(value).field("value", Rc::new(4_u8)))
        .unwrap();
    assert_eq!(*updated.value, 4);
    assert_eq!(*updated.value, 4);
}

#[test]
fn test_local_only_enum_is_reflected() {
    let descriptor = TypeDescriptor::of::<LocalEvent>();
    let variant = descriptor.variant("Value").unwrap();
    let field = variant.field("0").unwrap();
    let mut event = LocalEvent::Value(Rc::new(5_u8));
    assert_eq!(**field.get::<_, Rc<u8>>(&event).unwrap(), 5);
    field.set(&mut event, Rc::new(6_u8)).unwrap();
    let constructed: LocalEvent = variant
        .construct(qubit_reflect::construct::PositionalFields::<Local>::new().value(Rc::new(7_u8)))
        .unwrap();
    assert!(variant.is_active(&constructed).unwrap());
    assert_eq!(**field.get::<_, Rc<u8>>(&constructed).unwrap(), 7);
}

#[Reflect]
struct ThreadSafeRecord {
    pub value: u32,
}

#[Reflect]
struct ConditionalTuple(#[cfg(any())] u8, #[cfg_attr(all(), cfg(any()))] u16, pub u32);

#[Reflect]
struct ConditionalNestedTuple(
    #[cfg_attr(all(), cfg_attr(all(), cfg(any())))] u8,
    #[cfg(feature = "derive")]
    #[cfg_attr(all(), cfg(any()))]
    u16,
    pub u32,
);

#[test]
fn test_ordinary_types_keep_thread_safe_access_and_construction() {
    let descriptor = TypeDescriptor::of::<ThreadSafeRecord>();
    let field = descriptor.field("value").unwrap();
    let mut value = ThreadSafeRecord { value: 6 };
    assert_eq!(*field.get::<_, u32>(&value).unwrap(), 6);
    field.set(&mut value, 7_u32).unwrap();
    let _made: ThreadSafeRecord = descriptor
        .construct(NamedFields::<ThreadSafe>::new().field("value", 8_u32))
        .unwrap();
}

#[test]
fn test_cfg_tuple_survivor_is_accessible_by_source_index() {
    let descriptor = TypeDescriptor::of::<ConditionalTuple>();
    let field = descriptor.field("2").unwrap();
    let mut value = ConditionalTuple(7);
    assert_eq!(*field.get::<_, u32>(&value).unwrap(), 7);
    field.set(&mut value, 9_u32).unwrap();
    assert_eq!(value.0, 9);
}

#[test]
fn test_nested_and_multiple_tuple_cfg_attributes_preserve_source_index_access() {
    let descriptor = TypeDescriptor::of::<ConditionalNestedTuple>();
    let field = descriptor.field("2").unwrap();
    let mut value = ConditionalNestedTuple(11_u32);
    assert_eq!(*field.get::<_, u32>(&value).unwrap(), 11);
    field.set(&mut value, 13_u32).unwrap();
    assert_eq!(value.0, 13);
}
