// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::TypeId;

use qubit_reflect::OptionalProjectionError;
use qubit_reflect::ReflectedRef;
use qubit_reflect::TypeDescriptor;

#[test]
fn test_builtin_optional_projection_handles_some_none_and_mismatch() {
    let some = Some(17_u32);
    let none: Option<u32> = None;
    let descriptor = TypeDescriptor::of::<Option<u32>>()
        .as_optional()
        .expect("optional view");

    assert!(descriptor.has_ref_projection());
    assert_eq!(
        descriptor
            .project_ref(ReflectedRef::new(&some))
            .expect("project Some")
            .unwrap()
            .downcast_ref::<u32>(),
        Some(&17)
    );
    assert!(
        descriptor
            .project_ref(ReflectedRef::new(&none))
            .expect("project None")
            .is_none()
    );
    let Err(mismatch) = descriptor.project_ref(ReflectedRef::new(&Some(1_u8))) else {
        panic!("different Option specialization must fail")
    };
    let OptionalProjectionError::TypeMismatch(mismatch) = mismatch else {
        panic!("type mismatch")
    };
    assert_eq!(mismatch.expected(), TypeId::of::<Option<u32>>());
    assert_eq!(mismatch.actual(), TypeId::of::<Option<u8>>());

    let Err(string_mismatch) = descriptor.project_ref(ReflectedRef::new_str("x")) else {
        panic!("str must not match an optional descriptor")
    };
    let OptionalProjectionError::TypeMismatch(string_mismatch) = string_mismatch else {
        panic!("type mismatch")
    };
    assert_eq!(string_mismatch.expected(), TypeId::of::<Option<u32>>());
    assert_eq!(string_mismatch.actual(), TypeId::of::<str>());
}

#[test]
fn test_optional_projection_supports_nested_options_and_str_element() {
    let nested = Some(Some(23_u8));
    let outer = TypeDescriptor::of::<Option<Option<u8>>>()
        .as_optional()
        .expect("outer optional");
    let inner = outer
        .project_ref(ReflectedRef::new(&nested))
        .expect("project outer")
        .expect("outer Some");
    let inner_descriptor = TypeDescriptor::of::<Option<u8>>()
        .as_optional()
        .expect("inner optional");
    let element = inner_descriptor
        .project_ref(inner)
        .expect("project inner")
        .expect("inner Some");
    assert_eq!(element.downcast_ref::<u8>(), Some(&23));

    let text = Some("borrowed text");
    let descriptor = TypeDescriptor::of::<Option<&'static str>>()
        .as_optional()
        .expect("text optional");
    assert_eq!(
        descriptor
            .project_ref(ReflectedRef::new(&text))
            .expect("project text")
            .expect("Some")
            .downcast_ref::<&str>(),
        Some(&"borrowed text"),
    );
}
