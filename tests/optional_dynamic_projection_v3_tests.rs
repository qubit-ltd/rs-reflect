// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::OptionalProjectionError;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::value::DynamicMut;
use qubit_reflect::value::DynamicRef;
use qubit_reflect::value::Local;

#[test]
fn dynamic_shared_projection_preserves_type_and_some_semantics() {
    let descriptor = TypeDescriptor::of::<Option<u32>>();
    let optional = descriptor.as_optional().expect("Option descriptor");

    let value = Some(17_u32);
    let projected = optional
        .project_dynamic_ref(DynamicRef::<Local>::new(&value))
        .expect("matching option type")
        .expect("Some value");
    assert_eq!(projected.downcast_ref::<u32>(), Some(&17));

    let absent = None::<u32>;
    assert!(
        optional
            .project_dynamic_ref(DynamicRef::<Local>::new(&absent))
            .expect("matching None type")
            .is_none()
    );

    let wrong_type = Some("text");
    assert!(matches!(
        optional.project_dynamic_ref(DynamicRef::<Local>::new(&wrong_type)),
        Err(OptionalProjectionError::TypeMismatch(_))
    ));
}

#[test]
fn dynamic_mutable_projection_preserves_exclusive_borrow() {
    let descriptor = TypeDescriptor::of::<Option<u32>>();
    let optional = descriptor.as_optional().expect("Option descriptor");

    let mut value = Some(17_u32);
    {
        let mut projected = optional
            .project_dynamic_mut(DynamicMut::<Local>::new(&mut value))
            .expect("matching option type")
            .expect("Some value");
        *projected.downcast_mut::<u32>().expect("matching element") = 29;
    }
    assert_eq!(value, Some(29));

    let mut absent = None::<u32>;
    assert!(
        optional
            .project_dynamic_mut(DynamicMut::<Local>::new(&mut absent))
            .expect("matching None type")
            .is_none()
    );

    let mut wrong_type = Some("text");
    assert!(matches!(
        optional.project_dynamic_mut(DynamicMut::<Local>::new(&mut wrong_type)),
        Err(OptionalProjectionError::TypeMismatch(_))
    ));
}
