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
