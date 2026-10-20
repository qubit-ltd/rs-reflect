// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Integration tests for borrowed dynamic values.
use std::any::TypeId;

use qubit_reflect::value::DynamicMut;
use qubit_reflect::value::DynamicRef;
use qubit_reflect::value::Local;

/// Confirms a failed shared downcast leaves the original borrow usable.
#[test]
fn test_ref_downcast_failure_keeps_the_borrow_usable() {
    let number = 42_u32;
    let value = DynamicRef::<Local>::new(&number);

    assert_eq!(value.downcast_ref::<String>(), None);
    assert_eq!(value.downcast_ref::<u32>(), Some(&42));
    assert!(value.clone().is::<u32>());
    assert_eq!(value.downcast_ref::<str>(), None);
}

/// Confirms a failed mutable downcast leaves the original borrow usable.
#[test]
fn test_mut_downcast_failure_keeps_the_borrow_usable() {
    let mut number = 42_u32;
    {
        let mut value = DynamicMut::<Local>::new(&mut number);

        assert!(value.is::<u32>());
        assert_eq!(value.downcast_ref::<str>(), None);
        assert_eq!(value.downcast_mut::<str>(), None);
        assert_eq!(value.downcast_mut::<String>(), None);
        *value
            .downcast_mut::<u32>()
            .expect("the mutable borrow remains usable after a failed downcast") = 43;
    }

    assert_eq!(number, 43);
}

/// Confirms shared and mutable borrows expose exact typed access.
#[test]
fn test_borrowed_typed_access_preserves_type_identity() {
    let mut number = 42_u32;
    {
        let value = DynamicRef::<Local>::new(&number);
        assert_eq!(value.downcast_ref::<u32>(), Some(&42));
    }

    {
        let mut value = DynamicMut::<Local>::new(&mut number);
        *value.downcast_mut::<u32>().expect("the stored borrow is a u32") = 43;
    }

    assert_eq!(number, 43);
}

/// Confirms consuming a shared wrapper exposes the original borrow lifetime
/// and returns the wrapper unchanged after an exact-type mismatch.
#[test]
fn test_ref_consuming_downcast_preserves_borrow_and_recovers_mismatch() {
    let number = 42_u32;
    let value = DynamicRef::<Local>::new(&number);
    let failure = value
        .downcast::<String>()
        .expect_err("a mismatched consuming downcast must fail");
    assert_eq!(failure.expected(), TypeId::of::<String>());
    assert_eq!(failure.actual(), TypeId::of::<u32>());
    let value = failure.into_value();

    let number_ref = value
        .downcast::<u32>()
        .unwrap_or_else(|_| panic!("an exact consuming downcast should return the original borrow"));
    assert_eq!(number_ref, &42);
}

/// Confirms consuming a mutable wrapper returns an exclusive borrow with the
/// original lifetime and preserves the wrapper after a mismatch.
#[test]
fn test_mut_consuming_downcast_preserves_borrow_and_recovers_mismatch() {
    let mut number = 42_u32;
    let value = DynamicMut::<Local>::new(&mut number);
    let failure = value
        .downcast::<String>()
        .expect_err("a mismatched consuming downcast must fail");
    assert_eq!(failure.expected(), TypeId::of::<String>());
    assert_eq!(failure.actual(), TypeId::of::<u32>());
    let value = failure.into_value();
    *value
        .downcast::<u32>()
        .unwrap_or_else(|_| panic!("an exact consuming downcast should return the original mutable borrow")) = 43;

    assert_eq!(number, 43);
}
