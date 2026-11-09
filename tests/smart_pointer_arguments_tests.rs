// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::rc::Rc;
use std::sync::Arc;

use qubit_reflect::ReflectionContext;
use qubit_reflect::TypeDescriptor;

fn assert_pointee(pointer: TypeDescriptor<'_>, pointee_type: std::any::TypeId) {
    let context = pointer.context();
    let arguments = pointer.generic_arguments();
    assert_eq!(arguments.len(), 1);
    assert!(std::ptr::eq(arguments.context(), context));

    let pointee = arguments
        .get_at(0)
        .expect("smart pointer has a pointee argument")
        .type_argument()
        .expect("pointee argument is a type");
    assert!(std::ptr::eq(pointee.context(), context));
    assert_eq!(
        pointee
            .find_concrete_type()
            .expect("pointee type is registered in the context")
            .type_id(),
        pointee_type
    );
}

#[test]
fn test_smart_pointer_descriptors_retain_the_exact_pointee_and_context() {
    let context = ReflectionContext::builder()
        .include_type::<Rc<String>>()
        .include_type::<Arc<String>>()
        .include_type::<Box<String>>()
        .include_type::<String>()
        .build()
        .expect("smart pointer registrations should be valid");

    assert_pointee(
        TypeDescriptor::of_in::<Rc<String>>(&context),
        std::any::TypeId::of::<String>(),
    );
    assert_pointee(
        TypeDescriptor::of_in::<Arc<String>>(&context),
        std::any::TypeId::of::<String>(),
    );
    assert_pointee(
        TypeDescriptor::of_in::<Box<String>>(&context),
        std::any::TypeId::of::<String>(),
    );
}

#[test]
fn test_smart_pointer_descriptors_retain_unsized_slice_pointees() {
    let context = ReflectionContext::builder()
        .include_type::<Rc<[u8]>>()
        .include_type::<Arc<[u8]>>()
        .include_type::<[u8]>()
        .build()
        .expect("slice pointee registrations should be valid");

    assert_pointee(
        TypeDescriptor::of_in::<Rc<[u8]>>(&context),
        std::any::TypeId::of::<[u8]>(),
    );
    assert_pointee(
        TypeDescriptor::of_in::<Arc<[u8]>>(&context),
        std::any::TypeId::of::<[u8]>(),
    );
}
