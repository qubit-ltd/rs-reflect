//! Integration tests for thread-safe dynamic value mode.

use qubit_reflect::value::{SendReflectedMut, SendReflectedRef};

/// Verifies that the thread-safe wrapper itself can cross a thread boundary.
fn assert_send<T: Send>(_: T) {}

#[test]
/// Confirms a thread-safe shared borrow crosses the mode boundary safely.
fn test_thread_safe_ref_can_be_downgraded_without_losing_the_borrow() {
    let number = 42_u32;
    let value = SendReflectedRef::new(&number);
    assert_send(value);

    let value = SendReflectedRef::new(&number).into_local();
    assert_eq!(value.downcast_ref::<u32>(), Some(&42));
}

#[test]
/// Confirms a thread-safe mutable borrow crosses the mode boundary safely.
fn test_thread_safe_mut_can_be_downgraded_without_losing_the_borrow() {
    let mut number = 42_u32;
    let mut value = SendReflectedMut::new(&mut number).into_local();
    *value
        .downcast_mut::<u32>()
        .expect("the mutable borrow survives the mode downgrade") = 43;
    drop(value);

    assert_eq!(number, 43);
}
