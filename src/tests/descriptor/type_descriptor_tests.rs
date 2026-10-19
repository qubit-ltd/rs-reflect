// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0 (the "License");
//    you may not use this file except in compliance with the License.
// =============================================================================

//! Unit coverage for the vNext structural descriptor runtime.

use crate::runtime::context::ReflectionContext;
use crate::runtime::descriptor::TypeDescriptor;

#[cfg(feature = "derive")]
#[crate::Reflect]
struct Record {
    id: u32,
    title: String,
}

#[cfg(feature = "derive")]
#[crate::Reflect]
struct GenericRecord<T> {
    item: T,
}

#[test]
#[cfg(feature = "derive")]
fn test_type_descriptor_binds_context_and_source_ordered_fields() {
    let descriptor = TypeDescriptor::of::<Record>();
    let context = ReflectionContext::global();

    assert!(std::ptr::eq(descriptor.context(), context));
    assert_eq!(descriptor.type_id(), std::any::TypeId::of::<Record>());
    let fields = descriptor.fields().expect("derived struct shape is available");
    assert_eq!(fields.len(), 2);
    assert_eq!(fields.get_at(0).unwrap().name(), "id");
    assert_eq!(fields.get_at(1).unwrap().name(), "title");
    assert_eq!(fields.get("id").unwrap().index(), 0);
}

#[test]
#[cfg(feature = "derive")]
fn test_unreflectable_generic_field_keeps_its_expression_without_a_descriptor() {
    let descriptor = TypeDescriptor::of::<GenericRecord<std::rc::Rc<()>>>();
    let field = descriptor
        .fields()
        .expect("derived generic struct shape is available")
        .get("item")
        .expect("source field is present");

    assert!(field.find_concrete_type().is_none());
    assert!(!format!("{:?}", field.type_expression()).is_empty());
}
