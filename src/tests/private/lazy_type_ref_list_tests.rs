// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Tests deferred type-reference list resolution and diagnostics.

use crate::__private::LazyTypeRef;
use crate::__private::LazyTypeRefList;
use crate::descriptor::TypeDescriptor;

/// Verifies list inspection does not resolve entries before navigation and
/// that navigation freezes source-order results.
#[test]
fn test_lazy_type_ref_list_resolves_once_in_source_order() {
    let references =
        Box::leak(vec![LazyTypeRef::resolved::<u8>(), LazyTypeRef::resolved::<String>()].into_boxed_slice());
    let list = Box::leak(Box::new(LazyTypeRefList::new(references)));

    assert_eq!(list.len(), 2);
    assert_eq!(
        format!("{list:?}"),
        "LazyTypeRefList { length: 2, state: \"<unresolved>\" }"
    );

    let first = list.get();
    let second = list.get();
    assert!(std::ptr::eq(first, second));
    assert!(std::ptr::eq(
        first[0].as_resolved().expect("u8 relationship should resolve"),
        TypeDescriptor::of::<u8>(),
    ));
    assert!(std::ptr::eq(
        first[1].as_resolved().expect("String relationship should resolve"),
        TypeDescriptor::of::<String>(),
    ));
    assert!(format!("{list:?}").starts_with("LazyTypeRefList([Resolved("));
}
