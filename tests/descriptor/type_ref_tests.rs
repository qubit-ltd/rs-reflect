// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::descriptor::TypeRef;
use qubit_reflect::Reflect;
use qubit_reflect::TypeDescriptor;

struct Bad;

impl Reflect for Bad {
    fn type_descriptor() -> &'static TypeDescriptor {
        TypeDescriptor::of::<u8>()
    }
}

#[test]
fn test_of_returns_root_descriptor_for_sized_type() {
    let reference = TypeRef::of::<u32>();
    let root = reference.as_resolved().expect("resolved root");

    assert!(std::ptr::eq(root, TypeDescriptor::of::<u32>()));
}

#[test]
fn test_of_returns_root_descriptor_for_unsized_type() {
    let reference = TypeRef::of::<str>();
    let root = reference.as_resolved().expect("resolved root");

    assert!(std::ptr::eq(root, TypeDescriptor::of::<str>()));
}

#[test]
fn test_of_rejects_wrong_manual_reflect_implementation() {
    assert!(std::panic::catch_unwind(TypeRef::of::<Bad>).is_err());
}
