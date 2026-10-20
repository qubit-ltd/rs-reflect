// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Integration tests for owned dynamic values.
use qubit_reflect::value::DynamicOwned;
use qubit_reflect::value::Local;
use qubit_reflect::value::ThreadSafe;

/// Confirms an owned downcast failure returns a usable original wrapper.
#[test]
fn test_downcast_owned_failure_returns_the_original_wrapper() {
    let value = DynamicOwned::<Local>::new(String::from("owned value"));
    let failure = value.downcast::<u32>().expect_err("a String must not downcast to u32");
    assert_eq!(failure.expected(), std::any::TypeId::of::<u32>());
    assert_eq!(failure.actual(), std::any::TypeId::of::<String>());
    let value = failure.into_value();

    assert!(value.is::<String>());
    let text = match value.downcast::<String>() {
        Ok(text) => text,
        Err(_) => panic!("the returned wrapper must still contain a String"),
    };
    assert_eq!(text, "owned value");
}

/// Confirms local owned values expose exact typed access and recovery.
#[test]
fn test_owned_typed_access_preserves_type_identity() {
    let mut value = DynamicOwned::<Local>::new(41_u32);

    assert_eq!(value.downcast_ref::<u32>(), Some(&41));
    assert_eq!(
        *value
            .downcast_mut::<u32>()
            .expect("the local owned value should downcast mutably"),
        41,
    );
    *value.downcast_mut::<u32>().expect("the stored value is a u32") = 42;
    assert_eq!(value.downcast_ref::<u32>(), Some(&42));
}

/// Confirms a thread-safe owned value can be irreversibly downgraded.
#[test]
fn test_thread_safe_owned_retains_typed_value() {
    let value = DynamicOwned::<ThreadSafe>::new(String::from("thread-safe"));
    assert_send_and_sync(&value);

    let text = match value.downcast::<String>() {
        Ok(text) => text,
        Err(_) => panic!("the downgraded wrapper must still contain a String"),
    };
    assert_eq!(text, "thread-safe");
}

/// Confirms thread-safe owned values expose all typed and `Any` inspection
/// paths.
#[test]
fn test_thread_safe_owned_typed_and_any_access_preserve_value() {
    let mut value = DynamicOwned::<ThreadSafe>::new(51_u32);
    assert!(value.is::<u32>());
    assert!(!value.is::<String>());
    assert_eq!(value.downcast_ref::<u32>(), Some(&51));
    *value
        .downcast_mut::<u32>()
        .expect("thread-safe owned value should downcast mutably") = 52;
    assert_eq!(value.downcast_ref::<u32>(), Some(&52));

    assert_eq!(value.downcast_ref::<u32>(), Some(&52));
}

/// Verifies that a value retains both thread-safety auto traits.
fn assert_send_and_sync<T: Send + Sync>(_: &T) {}
