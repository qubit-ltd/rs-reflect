//! Integration tests for thread-safe dynamic value mode.

use qubit_reflect::value::{SendReflectedMut, SendReflectedRef};

/// Verifies that a wrapper retains both thread-safety auto traits.
fn assert_send_and_sync<T: Send + Sync>(_: T) {}

/// Confirms a thread-safe shared borrow crosses the mode boundary safely.
#[test]
fn test_thread_safe_ref_can_be_downgraded_without_losing_the_borrow() {
    let number = 42_u32;
    assert_send_and_sync(SendReflectedRef::new(&number));

    let value = SendReflectedRef::new(&number).into_local();
    assert_eq!(value.downcast_ref::<u32>(), Some(&42));
}

/// Confirms a thread-safe mutable borrow crosses the mode boundary safely.
#[test]
fn test_thread_safe_mut_can_be_downgraded_without_losing_the_borrow() {
    let mut number = 42_u32;
    assert_send_and_sync(SendReflectedMut::new(&mut number));

    {
        let mut value = SendReflectedMut::new(&mut number).into_local();
        *value
            .downcast_mut::<u32>()
            .expect("the mutable borrow survives the mode downgrade") = 43;
    }

    assert_eq!(number, 43);
}

/// Confirms thread-safe `str` variants retain their dedicated accessors after downgrade.
#[test]
fn test_thread_safe_str_variants_can_be_downgraded() {
    let value = SendReflectedRef::new_str("hello");
    assert_send_and_sync(value);
    let value = SendReflectedRef::new_str("hello");
    assert_eq!(value.into_local().as_str(), Some("hello"));

    let mut text = String::from("hello");
    let value = SendReflectedMut::new_str_mut(text.as_mut_str());
    assert_send_and_sync(value);
    {
        let value = SendReflectedMut::new_str_mut(text.as_mut_str());
        let mut value = value.into_local();
        value
            .as_str_mut()
            .expect("the dedicated mutable str borrow survives the downgrade")
            .make_ascii_uppercase();
    }

    assert_eq!(text, "HELLO");
}

/// Confirms consuming thread-safe shared downcasts preserve both the original
/// borrow lifetime and wrapper recovery after an exact-type mismatch.
#[test]
fn test_thread_safe_ref_consuming_downcast_recovers_mismatch() {
    let number = 42_u32;
    let value = SendReflectedRef::new(&number);
    let value = match value.downcast::<String>() {
        Ok(_) => panic!("a mismatched consuming downcast must fail"),
        Err(value) => value,
    };

    let number_ref = value.downcast::<u32>().unwrap_or_else(|_| {
        panic!("an exact consuming downcast should return the thread-safe borrow")
    });
    assert_eq!(number_ref, &42);
}

/// Confirms consuming thread-safe mutable downcasts preserve the exclusive
/// borrow and recover the wrapper after a mismatch.
#[test]
fn test_thread_safe_mut_consuming_downcast_recovers_mismatch() {
    let mut number = 42_u32;
    let value = SendReflectedMut::new(&mut number);
    let value = match value.downcast::<String>() {
        Ok(_) => panic!("a mismatched consuming downcast must fail"),
        Err(value) => value,
    };
    *value.downcast::<u32>().unwrap_or_else(|_| {
        panic!("an exact consuming downcast should return the thread-safe mutable borrow")
    }) = 43;

    assert_eq!(number, 43);
}

/// Confirms consuming typed extraction keeps dedicated thread-safe `str`
/// variants intact when they cannot participate in `Any` downcasts.
#[test]
fn test_thread_safe_str_consuming_downcast_returns_original_wrapper() {
    let value = SendReflectedRef::new_str("hello");
    let value = match value.downcast::<String>() {
        Ok(_) => panic!("a dedicated str variant must not downcast through Any"),
        Err(value) => value,
    };
    assert_eq!(value.as_str(), Some("hello"));

    let mut text = String::from("hello");
    let value = SendReflectedMut::new_str_mut(text.as_mut_str());
    let mut value = match value.downcast::<String>() {
        Ok(_) => panic!("a dedicated mutable str variant must not downcast through Any"),
        Err(value) => value,
    };
    value
        .as_str_mut()
        .expect("the original mutable str wrapper should be returned")
        .make_ascii_uppercase();
    assert_eq!(text, "HELLO");
}
