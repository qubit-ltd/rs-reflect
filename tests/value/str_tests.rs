// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Integration tests for reflected `str` borrows.

use qubit_reflect::value::DynamicMut;
use qubit_reflect::value::DynamicRef;
use qubit_reflect::value::Local;

#[test]
fn shared_str_can_be_inspected_and_extracted_by_type() {
    let text = String::from("hello");
    let value = DynamicRef::<Local>::new(text.as_str());

    assert_eq!(value.downcast_ref::<str>(), Some("hello"));
    assert_eq!(value.clone().downcast::<str>().unwrap(), "hello");
}

#[test]
fn mutable_str_can_be_inspected_mutated_and_extracted_by_type() {
    let mut text = String::from("hello");
    {
        let mut value = DynamicMut::<Local>::new(text.as_mut_str());
        value
            .downcast_mut::<str>()
            .expect("a mutable str borrow has the exact reflected type")
            .make_ascii_uppercase();
    }
    assert_eq!(text, "HELLO");

    let mut text = String::from("hello");
    match DynamicMut::<Local>::new(text.as_mut_str()).downcast::<str>() {
        Ok(extracted) => extracted.make_ascii_uppercase(),
        Err(_) => panic!("a mutable str borrow must be extractable by its exact type"),
    }
    assert_eq!(text, "HELLO");
}

#[test]
fn failed_str_extraction_returns_the_original_borrow() {
    let number = 42_u32;
    let failure = DynamicRef::<Local>::new(&number)
        .downcast::<str>()
        .expect_err("a u32 borrow cannot be extracted as str");
    assert_eq!(failure.expected(), std::any::TypeId::of::<str>());
    assert_eq!(failure.actual(), std::any::TypeId::of::<u32>());
    assert_eq!(failure.into_value().downcast_ref::<u32>(), Some(&42));

    let mut number = 42_u32;
    let failure = DynamicMut::<Local>::new(&mut number)
        .downcast::<str>()
        .expect_err("a mutable u32 borrow cannot be extracted as str");
    assert_eq!(failure.expected(), std::any::TypeId::of::<str>());
    assert_eq!(failure.actual(), std::any::TypeId::of::<u32>());
    *failure
        .into_value()
        .downcast_mut::<u32>()
        .expect("the failed extraction must retain the mutable borrow") = 43;
    assert_eq!(number, 43);
}
