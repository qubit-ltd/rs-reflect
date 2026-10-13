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
use qubit_reflect::access::FieldAccessPolicy;
use qubit_reflect::identity::VisibilityKind;

#[Reflect]
pub struct PrivateFieldRecord {
    pub visible: u32,
    secret: String,
}

#[Reflect]
pub enum VisibilityEvent {
    Value { payload: String },
    Empty,
}

#[test]
fn field_visibility_distinguishes_declared_and_variant_inherited_boundaries() {
    let record_field = TypeDescriptor::of::<PrivateFieldRecord>()
        .field("secret")
        .expect("private struct field remains described");
    let visibility = record_field.visibility();
    assert!(!visibility.is_variant_inherited());
    assert_eq!(
        visibility.as_declared().map(|value| value.kind()),
        Some(VisibilityKind::Private)
    );
    assert!(std::ptr::eq(
        record_field.declaring_type(),
        TypeDescriptor::of::<PrivateFieldRecord>()
    ));
    assert_eq!(record_field.index(), 1);
    assert_eq!(record_field.rust_name(), Some("secret"));
    assert_eq!(record_field.query_name(), Some("secret"));
    let qubit_reflect::descriptor::TypeRef::Resolved(field_type) = record_field.field_type() else {
        panic!("String field type should resolve to its reflected descriptor");
    };
    assert!(std::ptr::eq(*field_type, TypeDescriptor::of::<String>()));
    assert_eq!(record_field.access_policy(), FieldAccessPolicy::ReadWrite);
    assert_eq!(record_field.variant_index(), None);
    assert_eq!(record_field.variant_rust_name(), None);

    let variant_field = TypeDescriptor::of::<VisibilityEvent>()
        .variant("Value")
        .expect("payload variant exists")
        .field("payload")
        .expect("variant field exists");
    let visibility = variant_field.visibility();
    assert!(visibility.is_variant_inherited());
    assert!(visibility.as_declared().is_none());
    assert!(std::ptr::eq(
        variant_field.declaring_type(),
        TypeDescriptor::of::<VisibilityEvent>()
    ));
    assert_eq!(variant_field.index(), 0);
    assert_eq!(variant_field.rust_name(), Some("payload"));
    assert_eq!(variant_field.query_name(), Some("payload"));
    let qubit_reflect::descriptor::TypeRef::Resolved(field_type) = variant_field.field_type() else {
        panic!("enum payload type should resolve to its reflected descriptor");
    };
    assert!(std::ptr::eq(*field_type, TypeDescriptor::of::<String>()));
    assert_eq!(variant_field.access_policy(), FieldAccessPolicy::ReadWrite);
    assert_eq!(variant_field.variant_index(), Some(0));
    assert_eq!(variant_field.variant_rust_name(), Some("Value"));
}

#[test]
fn typed_unsafe_field_access_can_read_borrow_and_replace_a_private_field() {
    let descriptor = TypeDescriptor::of::<PrivateFieldRecord>();
    let secret = descriptor.field("secret").expect("private field is described");
    let mut value = PrivateFieldRecord {
        visible: 7,
        secret: String::from("before"),
    };

    // SAFETY: this test owns the reflected value and intentionally exercises
    // the private field access API.
    let read =
        unsafe { secret.get_unsafe::<_, String>(&value) }.expect("unsafe shared access bypasses only field visibility");
    assert_eq!(read, "before");

    // SAFETY: the exclusive borrow is scoped to this mutation and the field
    // remains a String.
    {
        let secret = unsafe { secret.get_mut_unsafe::<_, String>(&mut value) }
            .expect("unsafe mutable access bypasses only field visibility");
        secret.push_str("-mutated");
    }
    assert_eq!(value.secret, "before-mutated");

    // SAFETY: replacement preserves the private field's declared String type.
    unsafe { secret.set_unsafe(&mut value, String::from("after")) }
        .expect("unsafe replacement bypasses only field visibility");
    assert_eq!(value.secret, "after");
    assert_eq!(value.visible, 7);
}
