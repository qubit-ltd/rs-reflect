// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::TypeId;

use qubit_reflect::value::DynamicMut;
use qubit_reflect::value::DynamicRef;
use qubit_reflect::value::Local;
use qubit_reflect::value::ThreadSafe;

#[test]
fn local_shared_borrow_reborrow_and_mismatch_preserve_the_original_value() {
    let value = String::from("local shared");
    let dynamic = DynamicRef::<Local>::new(&value);

    assert_eq!(dynamic.clone().type_id(), TypeId::of::<String>());
    assert!(dynamic.clone().is::<String>());
    assert_eq!(
        dynamic.downcast_ref::<String>().map(String::as_str),
        Some("local shared")
    );
    assert!(dynamic.downcast_ref::<u32>().is_none());
    assert_eq!(
        dynamic.reborrow().downcast_ref::<String>().map(String::as_str),
        Some("local shared")
    );

    let failure = dynamic.downcast::<u32>().expect_err("String cannot downcast to u32");
    assert_eq!(failure.expected(), TypeId::of::<u32>());
    assert_eq!(failure.actual(), TypeId::of::<String>());
    assert_eq!(
        failure
            .into_value()
            .downcast::<String>()
            .expect("mismatch retains the shared borrow"),
        "local shared"
    );
}

#[test]
fn thread_safe_shared_borrow_reborrow_and_mismatch_preserve_the_original_value() {
    let value = String::from("thread safe shared");
    let dynamic = DynamicRef::<ThreadSafe>::new(&value);

    assert_eq!(dynamic.clone().type_id(), TypeId::of::<String>());
    assert!(dynamic.clone().is::<String>());
    assert_eq!(
        dynamic.downcast_ref::<String>().map(String::as_str),
        Some("thread safe shared")
    );
    assert!(dynamic.downcast_ref::<u32>().is_none());
    assert_eq!(
        dynamic.reborrow().downcast_ref::<String>().map(String::as_str),
        Some("thread safe shared")
    );

    let failure = dynamic.downcast::<u32>().expect_err("String cannot downcast to u32");
    assert_eq!(failure.expected(), TypeId::of::<u32>());
    assert_eq!(failure.actual(), TypeId::of::<String>());
    assert_eq!(
        failure
            .into_value()
            .downcast::<String>()
            .expect("mismatch retains the shared borrow"),
        "thread safe shared"
    );
}

#[test]
fn local_mutable_borrow_reborrow_and_mismatch_recover_exclusive_access() {
    let mut value = String::from("local mutable");
    let mut dynamic = DynamicMut::<Local>::new(&mut value);

    assert_eq!(dynamic.type_id(), TypeId::of::<String>());
    assert!(dynamic.is::<String>());
    assert_eq!(
        dynamic.downcast_ref::<String>().map(String::as_str),
        Some("local mutable")
    );
    assert!(dynamic.downcast_ref::<u32>().is_none());
    assert_eq!(
        dynamic.as_ref().downcast_ref::<String>().map(String::as_str),
        Some("local mutable")
    );

    {
        let mut child = dynamic.reborrow();
        child
            .downcast_mut::<String>()
            .expect("reborrow keeps String type")
            .push_str(" child");
    }

    let failure = dynamic.downcast::<u32>().expect_err("String cannot downcast to u32");
    assert_eq!(failure.expected(), TypeId::of::<u32>());
    assert_eq!(failure.actual(), TypeId::of::<String>());
    failure
        .into_value()
        .downcast::<String>()
        .expect("mismatch retains the exclusive borrow")
        .push_str(" recovered");
    assert_eq!(value, "local mutable child recovered");
}

#[test]
fn thread_safe_mutable_borrow_reborrow_and_mismatch_recover_exclusive_access() {
    let mut value = String::from("thread safe mutable");
    let mut dynamic = DynamicMut::<ThreadSafe>::new(&mut value);

    assert_eq!(dynamic.type_id(), TypeId::of::<String>());
    assert!(dynamic.is::<String>());
    assert_eq!(
        dynamic.downcast_ref::<String>().map(String::as_str),
        Some("thread safe mutable")
    );
    assert!(dynamic.downcast_ref::<u32>().is_none());
    assert_eq!(
        dynamic.as_ref().downcast_ref::<String>().map(String::as_str),
        Some("thread safe mutable")
    );

    {
        let mut child = dynamic.reborrow();
        child
            .downcast_mut::<String>()
            .expect("reborrow keeps String type")
            .push_str(" child");
    }

    let failure = dynamic.downcast::<u32>().expect_err("String cannot downcast to u32");
    assert_eq!(failure.expected(), TypeId::of::<u32>());
    assert_eq!(failure.actual(), TypeId::of::<String>());
    failure
        .into_value()
        .downcast::<String>()
        .expect("mismatch retains the exclusive borrow")
        .push_str(" recovered");
    assert_eq!(value, "thread safe mutable child recovered");
}
