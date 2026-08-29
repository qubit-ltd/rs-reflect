//! Integration tests for `str` dynamic values.

use qubit_reflect::value::{ReflectedMut, ReflectedRef};

#[test]
/// Confirms shared `str` values use their dedicated non-`Any` variant.
fn test_shared_str_uses_its_dedicated_accessors() {
    let text = "hello";
    let value = ReflectedRef::new_str(text);

    assert_eq!(value.as_str(), Some("hello"));
    assert!(value.as_any().is_none());
}

#[test]
/// Confirms mutable `str` values use their dedicated non-`Any` variant.
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
