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
use qubit_reflect::descriptor::FieldAccessError;
use qubit_reflect::descriptor::FieldSetError;
use qubit_reflect::value::DynamicMut;
use qubit_reflect::value::DynamicOwned;
use qubit_reflect::value::DynamicRef;
use qubit_reflect::value::Local;

#[Reflect]
struct FieldTarget {
    pub value: u32,
    secret: String,
}

#[test]
fn dynamic_field_access_borrows_and_recovers_replacements() {
    let field = TypeDescriptor::of::<FieldTarget>()
        .field("value")
        .expect("public field is reflected");
    let mut target = FieldTarget {
        value: 21,
        secret: String::from("private"),
    };

    let borrowed = field
        .get_dynamic(DynamicRef::<Local>::new(&target))
        .expect("dynamic shared access succeeds");
    assert_eq!(borrowed.downcast_ref::<u32>(), Some(&21));
    drop(borrowed);

    {
        let mut borrowed = field
            .get_dynamic_mut(DynamicMut::<Local>::new(&mut target))
            .expect("dynamic mutable access succeeds");
        *borrowed.downcast_mut::<u32>().expect("exact field type") = 22;
    }
    assert_eq!(target.value, 22);

    field
        .set_dynamic(
            DynamicMut::<Local>::new(&mut target),
            DynamicOwned::<Local>::new(23_u32),
        )
        .expect("dynamic replacement succeeds");
    assert_eq!(target.value, 23);

    let failure = field
        .set_dynamic(
            DynamicMut::<Local>::new(&mut target),
            DynamicOwned::<Local>::new(String::from("recover me")),
        )
        .expect_err("dynamic replacement checks the exact value type");
    assert!(matches!(failure.error(), FieldSetError::ValueTypeMismatch(_)));
    let (_, recovered) = failure.into_parts();
    assert_eq!(recovered.downcast::<String>().unwrap(), "recover me");
    assert_eq!(target.value, 23);

    let error = match field.get_dynamic(DynamicRef::<Local>::new(&17_u32)) {
        Err(error) => error,
        Ok(_) => panic!("dynamic access checks the declaring type"),
    };
    assert!(matches!(error, FieldAccessError::TargetTypeMismatch(_)));
}

#[test]
fn dynamic_field_access_enforces_source_visibility() {
    let field = TypeDescriptor::of::<FieldTarget>()
        .field("secret")
        .expect("private field is reflected");
    let mut target = FieldTarget {
        value: 1,
        secret: String::from("private"),
    };

    assert!(matches!(
        field.get_dynamic(DynamicRef::<Local>::new(&target)),
        Err(FieldAccessError::NotPublic)
    ));
    let failure = field
        .set_dynamic(
            DynamicMut::<Local>::new(&mut target),
            DynamicOwned::<Local>::new(String::from("denied")),
        )
        .expect_err("dynamic replacement respects source visibility");
    assert!(matches!(failure.error(), FieldSetError::NotPublic));
    let (_, recovered) = failure.into_parts();
    assert_eq!(recovered.downcast::<String>().unwrap(), "denied");
    assert_eq!(target.secret, "private");
}
