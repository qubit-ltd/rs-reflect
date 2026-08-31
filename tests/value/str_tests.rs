// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow explicit-imports
//! Integration tests for `str` dynamic values.
use qubit_reflect as reflect;
use reflect::value::ReflectedMut;
use reflect::value::ReflectedRef;

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

/// Confirms consuming local `str` extraction returns the original shared
/// borrow and preserves a non-`str` wrapper on failure.
#[test]
fn test_local_shared_str_consuming_extraction_preserves_borrow_and_wrapper() {
    let text = String::from("hello");
    let extracted = match ReflectedRef::new_str(text.as_str()).into_str() {
        Ok(extracted) => extracted,
        Err(_) => panic!("a dedicated str wrapper must extract successfully"),
    };
    assert_eq!(extracted, "hello");

    let number = 42_u32;
    let value = match ReflectedRef::new(&number).into_str() {
        Ok(_) => panic!("an Any-compatible wrapper must not extract as str"),
        Err(value) => value,
    };
    assert_eq!(value.downcast_ref::<u32>(), Some(&42));
}

/// Confirms consuming local mutable `str` extraction returns the original
/// exclusive borrow and preserves a non-`str` wrapper on failure.
#[test]
fn test_local_mutable_str_consuming_extraction_preserves_borrow_and_wrapper() {
    let mut text = String::from("hello");
    match ReflectedMut::new_str_mut(text.as_mut_str()).into_str_mut() {
        Ok(extracted) => extracted.make_ascii_uppercase(),
        Err(_) => panic!("a dedicated mutable str wrapper must extract successfully"),
    }
    assert_eq!(text, "HELLO");

    let mut number = 42_u32;
    let mut value = match ReflectedMut::new(&mut number).into_str_mut() {
        Ok(_) => panic!("an Any-compatible wrapper must not extract as mutable str"),
        Err(value) => value,
    };
    *value
        .downcast_mut::<u32>()
        .expect("failed str extraction must return the original wrapper") = 43;
    assert_eq!(number, 43);
}
