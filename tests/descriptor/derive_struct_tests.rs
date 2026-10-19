// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0 (the "License");
//    you may not use this file except in compliance with the License.
// =============================================================================

//! Integration tests for vNext `Reflect` struct metadata.

use std::rc::Rc;

use qubit_reflect::Reflect;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::descriptor::{StructKind, TypeKind};

#[Reflect]
struct NamedRecord {
    visible: u8,
    #[reflect(skip)]
    ignored: String,
    retained: bool,
}

#[Reflect]
struct TupleRecord(u8, String);

#[Reflect]
struct NewtypeRecord(u16);

#[Reflect]
struct UnitRecord;

#[Reflect]
struct RecursiveRecord {
    next: Option<Box<RecursiveRecord>>,
}

#[Reflect]
struct GenericRecord<T> {
    item: T,
}

/// Verifies a derived named struct exposes source ordered fields and preserves
/// source indexes when a field is skipped.
#[test]
fn test_derive_struct_preserves_named_fields_and_source_positions() {
    let descriptor = TypeDescriptor::of::<NamedRecord>();
    let fields = descriptor.fields();

    assert!(matches!(descriptor.kind(), TypeKind::Struct(view) if view.kind() == StructKind::Named));
    assert_eq!(fields.len(), 2);
    assert_eq!(fields.get("visible").unwrap().index(), 0);
    assert_eq!(fields.get("visible").unwrap().name(), "visible");
    assert_eq!(fields.get("retained").unwrap().index(), 2);
    assert!(fields.get("ignored").is_none());
    assert!(fields.get_at(1).is_none());
}

/// Verifies tuple, newtype, and unit structs retain distinct structural kinds.
#[test]
fn test_derive_struct_preserves_tuple_newtype_and_unit_shapes() {
    let tuple = TypeDescriptor::of::<TupleRecord>();
    let newtype = TypeDescriptor::of::<NewtypeRecord>();
    let unit = TypeDescriptor::of::<UnitRecord>();

    assert!(matches!(tuple.kind(), TypeKind::Struct(view) if view.kind() == StructKind::Tuple));
    assert!(matches!(newtype.kind(), TypeKind::Struct(view) if view.kind() == StructKind::Newtype));
    assert!(matches!(unit.kind(), TypeKind::Struct(view) if view.kind() == StructKind::Unit));
    assert_eq!(tuple.fields().get_at(0).unwrap().name(), "0");
    assert_eq!(tuple.fields().get_at(1).unwrap().name(), "1");
    assert_eq!(newtype.fields().len(), 1);
    assert!(unit.fields().is_empty());
}

/// Verifies recursive source expressions do not require recursively resolving
/// nested descriptors during registration.
#[test]
fn test_derive_struct_preserves_recursive_field_expression() {
    let descriptor = TypeDescriptor::of::<RecursiveRecord>();
    let field = descriptor.field("next").expect("recursive field is described");

    assert_eq!(field.index(), 0);
    assert!(format!("{}", field.type_expression()).contains("RecursiveRecord"));
}

/// Verifies generic field expressions do not impose a `Reflect` bound on the
/// field type and expose a concrete type only when available.
#[test]
fn test_derive_struct_preserves_unreflectable_generic_field() {
    let descriptor = TypeDescriptor::of::<GenericRecord<Rc<()>>>();
    let field = descriptor.field("item").expect("generic field is described");

    assert!(field.find_concrete_type().is_none());
    assert!(format!("{}", field.type_expression()).contains("Rc"));
}

/// Verifies concurrent lookup preserves the concrete type identity.
#[test]
fn test_derive_struct_descriptor_is_stable_under_concurrent_lookup() {
    let workers: Vec<_> = (0..8)
        .map(|_| std::thread::spawn(TypeDescriptor::of::<NamedRecord>))
        .collect();

    for worker in workers {
        let descriptor = worker.join().expect("descriptor lookup must not panic");
        assert_eq!(descriptor.type_id(), std::any::TypeId::of::<NamedRecord>());
    }
}
