// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Integration tests for thread-safe dynamic value mode.

use qubit_reflect::value::DynamicMut;
use qubit_reflect::value::DynamicOwned;
use qubit_reflect::value::DynamicRef;
use qubit_reflect::value::ThreadSafe;

fn assert_send_and_sync<T: Send + Sync>(_: &T) {}

#[test]
fn thread_safe_borrows_are_send_sync_and_preserve_mutation() {
    let number = 42_u32;
    let shared = DynamicRef::<ThreadSafe>::new(&number);
    assert_send_and_sync(&shared);
    assert_eq!(shared.downcast_ref::<u32>(), Some(&42));

    let mut number = 42_u32;
    {
        let mut mutable = DynamicMut::<ThreadSafe>::new(&mut number);
        assert_send_and_sync(&mutable);
        *mutable
            .downcast_mut::<u32>()
            .expect("the thread-safe borrow retains its exact type") = 43;
    }
    assert_eq!(number, 43);
}

#[test]
fn thread_safe_owned_values_are_send_sync_and_keep_typed_access() {
    let mut value = DynamicOwned::<ThreadSafe>::new(41_u32);
    assert_send_and_sync(&value);
    assert!(value.is::<u32>());
    assert!(!value.is::<String>());
    assert_eq!(value.downcast_ref::<u32>(), Some(&41));
    *value
        .downcast_mut::<u32>()
        .expect("the thread-safe owned value should downcast mutably") = 42;
    assert_eq!(value.downcast_ref::<u32>(), Some(&42));
}

#[test]
fn thread_safe_owned_borrow_bridges_preserve_the_mode_and_value() {
    let mut value = DynamicOwned::<ThreadSafe>::new(61_u32);
    let shared: DynamicRef<'_, ThreadSafe> = value.as_ref();
    assert_send_and_sync(&shared);
    assert_eq!(shared.downcast_ref::<u32>(), Some(&61));
    drop(shared);

    let mut mutable: DynamicMut<'_, ThreadSafe> = value.as_mut();
    assert_send_and_sync(&mutable);
    *mutable
        .downcast_mut::<u32>()
        .expect("the owned value must expose a typed mutable borrow") = 67;
    drop(mutable);
    assert_eq!(value.downcast_ref::<u32>(), Some(&67));
}

#[test]
fn thread_safe_str_borrows_support_typed_access_and_mutation() {
    let text = String::from("hello");
    let shared = DynamicRef::<ThreadSafe>::new(text.as_str());
    assert_send_and_sync(&shared);
    assert_eq!(shared.downcast_ref::<str>(), Some("hello"));

    let mut text = String::from("hello");
    {
        let mut mutable = DynamicMut::<ThreadSafe>::new(text.as_mut_str());
        assert_send_and_sync(&mutable);
        mutable
            .downcast_mut::<str>()
            .expect("thread-safe str remains available through typed access")
            .make_ascii_uppercase();
    }
    assert_eq!(text, "HELLO");
}
