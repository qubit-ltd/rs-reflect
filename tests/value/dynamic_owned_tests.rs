//! Integration tests for owned dynamic values.

use qubit_reflect::value::{ReflectedOwned, SendReflectedOwned};

/// Confirms an owned downcast failure returns a usable original wrapper.
#[test]
fn test_downcast_owned_failure_returns_the_original_wrapper() {
    let value = ReflectedOwned::new(String::from("owned value"));
    let value = value
        .downcast::<u32>()
        .expect_err("a String must not downcast to u32");

    assert!(value.is::<String>());
    let text = match value.downcast::<String>() {
        Ok(text) => text,
        Err(_) => panic!("the returned wrapper must still contain a String"),
    };
    assert_eq!(text, "owned value");
}

/// Confirms local owned values expose safe `Any` interoperability.
#[test]
fn test_owned_any_interoperation_preserves_type_identity() {
    let mut value = ReflectedOwned::new(41_u32);

    assert_eq!(
        value.as_any().and_then(|value| value.downcast_ref::<u32>()),
        Some(&41)
    );
    *value
        .as_any_mut()
        .and_then(|value| value.downcast_mut::<u32>())
        .expect("the stored value is a u32") = 42;
    let value = match value.into_any() {
        Ok(value) => value,
        Err(_) => panic!("owned values are Any-compatible"),
    };
    let value = match value.downcast::<u32>() {
        Ok(value) => value,
        Err(_) => panic!("the stored value remains a u32"),
    };
    assert_eq!(*value, 42);
}

/// Confirms a thread-safe owned value can be irreversibly downgraded.
#[test]
fn test_thread_safe_owned_can_be_downgraded_to_local() {
    let value = SendReflectedOwned::new(String::from("thread-safe"));
    assert_send_and_sync(&value);
    let value = value.into_local();

    let text = match value.downcast::<String>() {
        Ok(text) => text,
        Err(_) => panic!("the downgraded wrapper must still contain a String"),
    };
    assert_eq!(text, "thread-safe");
}

/// Verifies that a value retains both thread-safety auto traits.
fn assert_send_and_sync<T: Send + Sync>(_: &T) {}
