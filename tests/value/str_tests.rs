//! Integration tests for `str` dynamic values.

use qubit_reflect::value::{ReflectedMut, ReflectedRef};

/// Confirms shared `str` values use their dedicated non-`Any` variant.
#[test]
fn test_shared_str_uses_its_dedicated_accessors() {
    let text = "hello";
    let value = ReflectedRef::new_str(text);

    assert_eq!(value.as_str(), Some("hello"));
    assert!(value.as_any().is_none());
}

/// Confirms mutable `str` values use their dedicated non-`Any` variant.
#[test]
fn test_mutable_str_uses_its_dedicated_accessors() {
    let mut text = String::from("hello");
    let mut value = ReflectedMut::new_str_mut(text.as_mut_str());

    value
        .as_str_mut()
        .expect("a mutable str value exposes its dedicated accessor")
        .make_ascii_uppercase();
    assert_eq!(value.as_str(), Some("HELLO"));
    assert!(value.as_any().is_none());
}

/// Confirms consuming local typed extraction returns dedicated `str` wrappers
/// unchanged because they do not participate in `Any` downcasts.
#[test]
fn test_local_str_consuming_downcast_returns_original_wrapper() {
    let value = ReflectedRef::new_str("hello");
    let value = match value.downcast::<String>() {
        Ok(_) => panic!("a dedicated str variant must not downcast through Any"),
        Err(value) => value,
    };
    assert_eq!(value.as_str(), Some("hello"));

    let mut text = String::from("hello");
    let value = ReflectedMut::new_str_mut(text.as_mut_str());
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
