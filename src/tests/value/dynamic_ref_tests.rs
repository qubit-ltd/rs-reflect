// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::value::DynamicRef;
use crate::value::Local;
use crate::value::ThreadSafe;

#[test]
fn test_local_dynamic_ref_preserves_any_and_str_borrows() {
    let number = 42_u32;
    let value = DynamicRef::<Local>::new(&number);
    assert!(value.is::<u32>());
    assert_eq!(value.downcast_ref::<u32>(), Some(&number));
    assert!(value.as_any().is_some());
    assert!(value.as_str().is_none());
    assert_eq!(value.downcast::<u32>().ok(), Some(&number));

    let text = DynamicRef::<Local>::new_str("local");
    assert!(!text.is::<u32>());
    assert!(text.downcast_ref::<u32>().is_none());
    assert!(text.as_any().is_none());
    assert_eq!(text.as_str(), Some("local"));
    assert_eq!(text.into_str().ok(), Some("local"));
}

#[test]
fn test_thread_safe_dynamic_ref_preserves_borrows_when_downgraded() {
    let number = 7_u32;
    let value = DynamicRef::<ThreadSafe>::new(&number);
    assert!(value.is::<u32>());
    assert_eq!(value.downcast_ref::<u32>(), Some(&number));
    assert!(value.as_any().is_some());
    assert!(value.as_str().is_none());
    assert_eq!(value.downcast::<u32>().ok(), Some(&number));

    let text = DynamicRef::<ThreadSafe>::new_str("shared");
    assert!(!text.is::<u32>());
    assert!(text.downcast_ref::<u32>().is_none());
    assert!(text.as_any().is_none());
    assert_eq!(text.as_str(), Some("shared"));
    assert_eq!(text.into_str().ok(), Some("shared"));

    let downgraded = DynamicRef::<ThreadSafe>::new(&number).into_local();
    assert_eq!(downgraded.downcast::<u32>().ok(), Some(&number));
}
