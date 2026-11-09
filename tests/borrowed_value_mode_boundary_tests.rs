// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::TypeId;

use qubit_reflect::value::DynamicMut;
use qubit_reflect::value::DynamicRef;
use qubit_reflect::value::Local;
use qubit_reflect::value::ThreadSafe;

#[test]
fn test_local_shared_borrow_reborrow_and_mismatch_preserve_the_original_value() {
    let value = String::from("local shared");
    let dynamic = DynamicRef::<Local>::new(&value);

    assert_eq!(dynamic.clone().type_id(), TypeId::of::<String>());
    assert!(dynamic.clone().is::<String>());
    assert_eq!(
        dynamic.downcast_ref::<String>().map(String::as_str),
        Some("local shared")
    );
    assert!(dynamic.downcast_ref::<u32>().is_none());
    assert_eq!(
        dynamic.reborrow().downcast_ref::<String>().map(String::as_str),
        Some("local shared")
    );

    let failure = dynamic.downcast::<u32>().expect_err("String cannot downcast to u32");
    assert_eq!(failure.expected(), TypeId::of::<u32>());
    assert_eq!(failure.actual(), TypeId::of::<String>());
    assert_eq!(
        failure
            .into_value()
            .downcast::<String>()
            .expect("mismatch retains the shared borrow"),
        "local shared"
    );
}

#[test]
fn test_thread_safe_shared_borrow_reborrow_and_mismatch_preserve_the_original_value() {
    let value = String::from("thread safe shared");
    let dynamic = DynamicRef::<ThreadSafe>::new(&value);

    assert_eq!(dynamic.clone().type_id(), TypeId::of::<String>());
    assert!(dynamic.clone().is::<String>());
    assert_eq!(
        dynamic.downcast_ref::<String>().map(String::as_str),
        Some("thread safe shared")
    );
    assert!(dynamic.downcast_ref::<u32>().is_none());
    assert_eq!(
        dynamic.reborrow().downcast_ref::<String>().map(String::as_str),
        Some("thread safe shared")
    );

    let failure = dynamic.downcast::<u32>().expect_err("String cannot downcast to u32");
    assert_eq!(failure.expected(), TypeId::of::<u32>());
    assert_eq!(failure.actual(), TypeId::of::<String>());
    assert_eq!(
        failure
            .into_value()
            .downcast::<String>()
            .expect("mismatch retains the shared borrow"),
        "thread safe shared"
    );
}

#[test]
fn test_local_mutable_borrow_reborrow_and_mismatch_recover_exclusive_access() {
    let mut value = String::from("local mutable");
    let mut dynamic = DynamicMut::<Local>::new(&mut value);

    assert_eq!(dynamic.type_id(), TypeId::of::<String>());
    assert!(dynamic.is::<String>());
    assert_eq!(
        dynamic.downcast_ref::<String>().map(String::as_str),
        Some("local mutable")
    );
    assert!(dynamic.downcast_ref::<u32>().is_none());
    assert_eq!(
        dynamic.as_ref().downcast_ref::<String>().map(String::as_str),
        Some("local mutable")
    );

    {
        let mut child = dynamic.reborrow();
        child
            .downcast_mut::<String>()
            .expect("reborrow keeps String type")
            .push_str(" child");
    }

    let failure = dynamic.downcast::<u32>().expect_err("String cannot downcast to u32");
    assert_eq!(failure.expected(), TypeId::of::<u32>());
    assert_eq!(failure.actual(), TypeId::of::<String>());
    failure
        .into_value()
        .downcast::<String>()
        .expect("mismatch retains the exclusive borrow")
        .push_str(" recovered");
    assert_eq!(value, "local mutable child recovered");
}

#[test]
fn test_thread_safe_mutable_borrow_reborrow_and_mismatch_recover_exclusive_access() {
    let mut value = String::from("thread safe mutable");
    let mut dynamic = DynamicMut::<ThreadSafe>::new(&mut value);

    assert_eq!(dynamic.type_id(), TypeId::of::<String>());
    assert!(dynamic.is::<String>());
    assert_eq!(
        dynamic.downcast_ref::<String>().map(String::as_str),
        Some("thread safe mutable")
    );
    assert!(dynamic.downcast_ref::<u32>().is_none());
    assert_eq!(
        dynamic.as_ref().downcast_ref::<String>().map(String::as_str),
        Some("thread safe mutable")
    );

    {
        let mut child = dynamic.reborrow();
        child
            .downcast_mut::<String>()
            .expect("reborrow keeps String type")
            .push_str(" child");
    }

    let failure = dynamic.downcast::<u32>().expect_err("String cannot downcast to u32");
    assert_eq!(failure.expected(), TypeId::of::<u32>());
    assert_eq!(failure.actual(), TypeId::of::<String>());
    failure
        .into_value()
        .downcast::<String>()
        .expect("mismatch retains the exclusive borrow")
        .push_str(" recovered");
    assert_eq!(value, "thread safe mutable child recovered");
}

#[test]
fn test_local_shared_dst_borrows_clone_and_mismatch_retain_the_original_value() {
    let text = String::from("local str");
    let text_ptr = text.as_ptr();
    let dynamic = DynamicRef::<Local>::new(text.as_str());
    let clone = dynamic.clone();
    assert_eq!(clone.downcast_ref::<str>().unwrap(), "local str");

    let failure = dynamic.downcast::<u8>().expect_err("str cannot downcast to u8");
    assert_eq!(failure.expected(), TypeId::of::<u8>());
    assert_eq!(failure.actual(), TypeId::of::<str>());
    let recovered = failure
        .into_value()
        .downcast::<str>()
        .expect("mismatch retains the shared str borrow");
    assert_eq!(recovered.as_ptr(), text_ptr);
    assert_eq!(recovered, "local str");

    let bytes = [4_u8, 5, 6];
    let bytes_ptr = bytes.as_ptr();
    let dynamic = DynamicRef::<Local>::new(&bytes[..]);
    assert_eq!(dynamic.clone().downcast_ref::<[u8]>(), Some(&bytes[..]));
    let failure = dynamic.downcast::<str>().expect_err("[u8] cannot downcast to str");
    assert_eq!(failure.expected(), TypeId::of::<str>());
    assert_eq!(failure.actual(), TypeId::of::<[u8]>());
    let recovered = failure
        .into_value()
        .downcast::<[u8]>()
        .expect("mismatch retains the shared slice borrow");
    assert_eq!(recovered.as_ptr(), bytes_ptr);
    assert_eq!(recovered, &[4, 5, 6]);
}

#[test]
fn test_thread_safe_shared_dst_borrows_clone_and_mismatch_retain_the_original_value() {
    let text = String::from("thread safe str");
    let text_ptr = text.as_ptr();
    let dynamic = DynamicRef::<ThreadSafe>::new(text.as_str());
    let clone = dynamic.clone();
    assert_eq!(clone.downcast_ref::<str>().unwrap(), "thread safe str");

    let failure = dynamic.downcast::<u8>().expect_err("str cannot downcast to u8");
    assert_eq!(failure.expected(), TypeId::of::<u8>());
    assert_eq!(failure.actual(), TypeId::of::<str>());
    let recovered = failure
        .into_value()
        .downcast::<str>()
        .expect("mismatch retains the shared str borrow");
    assert_eq!(recovered.as_ptr(), text_ptr);
    assert_eq!(recovered, "thread safe str");

    let bytes = [7_u8, 8, 9];
    let bytes_ptr = bytes.as_ptr();
    let dynamic = DynamicRef::<ThreadSafe>::new(&bytes[..]);
    assert_eq!(dynamic.clone().downcast_ref::<[u8]>(), Some(&bytes[..]));
    let failure = dynamic.downcast::<str>().expect_err("[u8] cannot downcast to str");
    assert_eq!(failure.expected(), TypeId::of::<str>());
    assert_eq!(failure.actual(), TypeId::of::<[u8]>());
    let recovered = failure
        .into_value()
        .downcast::<[u8]>()
        .expect("mismatch retains the shared slice borrow");
    assert_eq!(recovered.as_ptr(), bytes_ptr);
    assert_eq!(recovered, &[7, 8, 9]);
}

#[test]
fn test_local_mutable_dst_views_and_nested_reborrows_restore_exclusive_access() {
    let mut text = String::from("local str");
    let text_ptr = text.as_ptr();
    let mut dynamic = DynamicMut::<Local>::new(text.as_mut_str());
    {
        let shared = dynamic.as_ref();
        let clone = shared.clone();
        let nested = clone.reborrow();
        assert_eq!(nested.downcast_ref::<str>(), Some("local str"));
    }
    {
        let mut outer = dynamic.reborrow();
        {
            let mut inner = outer.reborrow();
            inner
                .downcast_mut::<str>()
                .expect("reborrow preserves str")
                .make_ascii_uppercase();
        }
        outer
            .downcast_mut::<str>()
            .expect("outer reborrow preserves str")
            .make_ascii_lowercase();
    }
    let failure = dynamic.downcast::<[u8]>().expect_err("str cannot downcast to [u8]");
    assert_eq!(failure.expected(), TypeId::of::<[u8]>());
    assert_eq!(failure.actual(), TypeId::of::<str>());
    {
        let recovered = failure
            .into_value()
            .downcast::<str>()
            .expect("mismatch retains the exclusive str borrow");
        assert_eq!(recovered.as_ptr(), text_ptr);
        recovered.make_ascii_uppercase();
    }
    assert_eq!(text, "LOCAL STR");

    let mut bytes = [1_u8, 2, 3];
    let bytes_ptr = bytes.as_ptr();
    let mut dynamic = DynamicMut::<Local>::new(&mut bytes[..]);
    {
        let shared = dynamic.as_ref();
        assert_eq!(shared.downcast_ref::<[u8]>(), Some(&[1, 2, 3][..]));
    }
    {
        let mut outer = dynamic.reborrow();
        {
            let mut inner = outer.reborrow();
            inner.downcast_mut::<[u8]>().expect("reborrow preserves [u8]")[0] = 9;
        }
        outer.downcast_mut::<[u8]>().expect("outer reborrow preserves [u8]")[1] = 8;
    }
    let failure = dynamic.downcast::<str>().expect_err("[u8] cannot downcast to str");
    assert_eq!(failure.expected(), TypeId::of::<str>());
    assert_eq!(failure.actual(), TypeId::of::<[u8]>());
    {
        let recovered = failure
            .into_value()
            .downcast::<[u8]>()
            .expect("mismatch retains the exclusive slice borrow");
        assert_eq!(recovered.as_ptr(), bytes_ptr);
        recovered[2] = 7;
    }
    assert_eq!(bytes, [9, 8, 7]);
}

#[test]
fn test_thread_safe_mutable_dst_views_and_nested_reborrows_restore_exclusive_access() {
    let mut text = String::from("thread safe str");
    let text_ptr = text.as_ptr();
    let mut dynamic = DynamicMut::<ThreadSafe>::new(text.as_mut_str());
    {
        let shared = dynamic.as_ref();
        let clone = shared.clone();
        let nested = clone.reborrow();
        assert_eq!(nested.downcast_ref::<str>(), Some("thread safe str"));
    }
    {
        let mut outer = dynamic.reborrow();
        {
            let mut inner = outer.reborrow();
            inner
                .downcast_mut::<str>()
                .expect("reborrow preserves str")
                .make_ascii_uppercase();
        }
        outer
            .downcast_mut::<str>()
            .expect("outer reborrow preserves str")
            .make_ascii_lowercase();
    }
    let failure = dynamic.downcast::<[u8]>().expect_err("str cannot downcast to [u8]");
    assert_eq!(failure.expected(), TypeId::of::<[u8]>());
    assert_eq!(failure.actual(), TypeId::of::<str>());
    {
        let recovered = failure
            .into_value()
            .downcast::<str>()
            .expect("mismatch retains the exclusive str borrow");
        assert_eq!(recovered.as_ptr(), text_ptr);
        recovered.make_ascii_uppercase();
    }
    assert_eq!(text, "THREAD SAFE STR");

    let mut bytes = [4_u8, 5, 6];
    let bytes_ptr = bytes.as_ptr();
    let mut dynamic = DynamicMut::<ThreadSafe>::new(&mut bytes[..]);
    {
        let shared = dynamic.as_ref();
        assert_eq!(shared.downcast_ref::<[u8]>(), Some(&[4, 5, 6][..]));
    }
    {
        let mut outer = dynamic.reborrow();
        {
            let mut inner = outer.reborrow();
            inner.downcast_mut::<[u8]>().expect("reborrow preserves [u8]")[0] = 7;
        }
        outer.downcast_mut::<[u8]>().expect("outer reborrow preserves [u8]")[1] = 8;
    }
    let failure = dynamic.downcast::<str>().expect_err("[u8] cannot downcast to str");
    assert_eq!(failure.expected(), TypeId::of::<str>());
    assert_eq!(failure.actual(), TypeId::of::<[u8]>());
    {
        let recovered = failure
            .into_value()
            .downcast::<[u8]>()
            .expect("mismatch retains the exclusive slice borrow");
        assert_eq!(recovered.as_ptr(), bytes_ptr);
        recovered[2] = 9;
    }
    assert_eq!(bytes, [7, 8, 9]);
}
