// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::TypeId;

use qubit_reflect::OptionalProjectionError;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::value::DynamicMut;
use qubit_reflect::value::DynamicRef;
use qubit_reflect::value::Local;

#[test]
fn test_builtin_optional_projection_handles_some_none_and_mismatch() {
    let some = Some(17_u32);
    let none: Option<u32> = None;
    let descriptor = TypeDescriptor::of::<Option<u32>>()
        .as_optional()
        .expect("optional view");

    assert!(descriptor.has_ref_projection());
    assert_eq!(descriptor.project_ref(&some).expect("project Some").unwrap(), &17_u32);
    assert!(descriptor.project_ref(&none).expect("project None").is_none());
    let Err(mismatch) = descriptor.project_ref::<u8>(&Some(1_u8)) else {
        panic!("different Option specialization must fail")
    };
    let OptionalProjectionError::TypeMismatch(mismatch) = mismatch else {
        panic!("type mismatch")
    };
    assert_eq!(mismatch.expected(), TypeId::of::<Option<u32>>());
    assert_eq!(mismatch.actual(), TypeId::of::<Option<u8>>());
}

#[test]
fn test_optional_projection_supports_nested_options_and_str_element() {
    let nested = Some(Some(23_u8));
    let outer = TypeDescriptor::of::<Option<Option<u8>>>()
        .as_optional()
        .expect("outer optional");
    let inner = outer.project_ref(&nested).expect("project outer").expect("outer Some");
    let inner_descriptor = TypeDescriptor::of::<Option<u8>>()
        .as_optional()
        .expect("inner optional");
    let element = inner_descriptor
        .project_ref(inner)
        .expect("project inner")
        .expect("inner Some");
    assert_eq!(element, &23_u8);

    let text = Some("borrowed text");
    let descriptor = TypeDescriptor::of::<Option<&'static str>>()
        .as_optional()
        .expect("text optional");
    assert_eq!(
        descriptor.project_ref(&text).expect("project text").expect("Some"),
        &"borrowed text",
    );
}

#[test]
fn dynamic_optional_projection_handles_some_none_and_wrong_root_types() {
    let descriptor = TypeDescriptor::of::<Option<u32>>()
        .as_optional()
        .expect("optional view");

    let some = Some(31_u32);
    let projected = descriptor
        .project_dynamic_ref(DynamicRef::<Local>::new(&some))
        .expect("matching optional root")
        .expect("Some projects an element");
    assert_eq!(projected.downcast_ref::<u32>(), Some(&31));

    let none = None::<u32>;
    assert!(
        descriptor
            .project_dynamic_ref(DynamicRef::<Local>::new(&none))
            .expect("matching empty optional")
            .is_none()
    );

    let wrong = Some("text");
    let Err(OptionalProjectionError::TypeMismatch(mismatch)) =
        descriptor.project_dynamic_ref(DynamicRef::<Local>::new(&wrong))
    else {
        panic!("a different Option specialization must be rejected");
    };
    assert_eq!(mismatch.expected(), TypeId::of::<Option<u32>>());
    assert_eq!(mismatch.actual(), TypeId::of::<Option<&str>>());
}

#[test]
fn dynamic_mutable_optional_projection_preserves_mutation_and_empty_state() {
    let descriptor = TypeDescriptor::of::<Option<u32>>()
        .as_optional()
        .expect("optional view");
    let mut some = Some(31_u32);
    {
        let mut projected = descriptor
            .project_dynamic_mut(DynamicMut::<Local>::new(&mut some))
            .expect("matching optional root")
            .expect("Some projects an element");
        *projected.downcast_mut::<u32>().expect("exact element type") = 32;
    }
    assert_eq!(some, Some(32));

    let mut none = None::<u32>;
    assert!(
        descriptor
            .project_dynamic_mut(DynamicMut::<Local>::new(&mut none))
            .expect("matching empty optional")
            .is_none()
    );

    let mut wrong = Some("text");
    let Err(OptionalProjectionError::TypeMismatch(mismatch)) =
        descriptor.project_dynamic_mut(DynamicMut::<Local>::new(&mut wrong))
    else {
        panic!("a different Option specialization must be rejected");
    };
    assert_eq!(mismatch.expected(), TypeId::of::<Option<u32>>());
    assert_eq!(mismatch.actual(), TypeId::of::<Option<&str>>());
}
