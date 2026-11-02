// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Integration tests for owned dynamic values.
use qubit_reflect::Reflect;
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

static DROP_COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

#[Reflect]
struct DropProbe;

impl Drop for DropProbe {
    fn drop(&mut self) {
        DROP_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

#[repr(align(128))]
#[Reflect]
struct HighlyAligned(u8);

#[derive(Debug, PartialEq, Eq)]
#[Reflect]
struct ZeroSized;

#[test]
fn test_local_error_recovery_any_and_single_drop() {
    DROP_COUNT.store(0, std::sync::atomic::Ordering::SeqCst);
    let value = DynamicOwned::<Local>::new(DropProbe);
    assert!(value.as_any().downcast_ref::<DropProbe>().is_some());
    let failure = value
        .downcast::<String>()
        .expect_err("DropProbe must not downcast to String");
    let recovered = failure
        .into_value()
        .downcast::<DropProbe>()
        .unwrap_or_else(|_| panic!("error value must retain its original type"));
    assert_eq!(DROP_COUNT.load(std::sync::atomic::Ordering::SeqCst), 0);
    drop(recovered);
    assert_eq!(DROP_COUNT.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[test]
fn test_thread_safe_error_recovery_and_any_borrow() {
    let value = DynamicOwned::<ThreadSafe>::new(73_u64);
    assert_eq!(value.as_any().downcast_ref::<u64>(), Some(&73));
    let failure = value.downcast::<String>().expect_err("u64 must not downcast to String");
    let recovered = failure
        .into_value()
        .downcast::<u64>()
        .unwrap_or_else(|_| panic!("error value must retain its original type"));
    assert_eq!(recovered, 73);
}

#[test]
fn test_owned_recovery_handles_zero_sized_and_high_alignment_values() {
    assert_eq!(
        DynamicOwned::<Local>::new(ZeroSized).downcast::<ZeroSized>().ok(),
        Some(ZeroSized)
    );
    let value = DynamicOwned::<Local>::new(HighlyAligned(9));
    assert_eq!(
        (value.downcast_ref::<HighlyAligned>().unwrap() as *const HighlyAligned as usize) % 128,
        0
    );
    let recovered = value
        .downcast::<HighlyAligned>()
        .unwrap_or_else(|_| panic!("aligned value must be recoverable"));
    assert_eq!(recovered.0, 9);
}
