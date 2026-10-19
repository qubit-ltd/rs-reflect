// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0 (the "License");
//    you may not use this file except in compliance with the License.
// =============================================================================

//! Integration tests for vNext `Reflect` enum metadata.

use qubit_reflect::Reflect;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::access::{FieldAccessError, FieldSetError};
use qubit_reflect::descriptor::TypeKind;
use qubit_reflect::value::{DynamicRef, Local};

#[derive(Reflect)]
enum Event {
    Ready,
    Number(u32),
    Named { value: u8 },
    #[reflect(skip)]
    Hidden(String),
}

/// Verifies enum variants and their fields retain source names and positions.
#[test]
fn test_derive_enum_preserves_variant_and_field_metadata() {
    let descriptor = TypeDescriptor::of::<Event>();

    assert!(matches!(descriptor.kind(), TypeKind::Enum(_)));
    assert_eq!(descriptor.variants().len(), 3);
    assert_eq!(descriptor.variant("Ready").unwrap().index(), 0);
    assert_eq!(descriptor.variant("Number").unwrap().index(), 1);
    assert_eq!(descriptor.variant("Named").unwrap().fields().get("value").unwrap().name(), "value");
    assert!(descriptor.variants().get("Hidden").is_none());
    assert!(descriptor.variants().get_at(3).is_none());
}

/// Verifies a variant adapter accepts only its exact active variant.
#[test]
fn test_derive_enum_variant_activity_checks_typed_and_dynamic_values() {
    let descriptor = TypeDescriptor::of::<Event>();
    let ready = descriptor.variant("Ready").expect("Ready variant is described");
    let number = descriptor.variant("Number").expect("Number variant is described");

    assert!(ready.is_active(&Event::Ready).unwrap());
    assert!(!ready.is_active(&Event::Number(9)).unwrap());
    assert!(number
        .is_active_dynamic(DynamicRef::<Local>::new(&Event::Number(9)))
        .unwrap());
    assert!(!number
        .is_active_dynamic(DynamicRef::<Local>::new(&Event::Ready))
        .unwrap());
}

/// Verifies typed field access reports inactive variants before reaching the
/// field adapter and preserves a rejected replacement.
#[test]
fn test_derive_enum_field_access_preserves_inactive_variant_recovery() {
    let field = TypeDescriptor::of::<Event>()
        .variant("Number")
        .expect("Number variant is described")
        .field("0")
        .expect("tuple field is described");
    let mut value = Event::Ready;

    assert!(matches!(field.get::<_, u32>(&value), Err(FieldAccessError::InactiveVariant { .. })));
    let failure = field
        .set(&mut value, 12_u32)
        .expect_err("setting an inactive variant field must fail");
    assert!(matches!(failure.error(), FieldSetError::InactiveVariant { .. }));
    assert_eq!(failure.into_parts().1, 12_u32);
}
