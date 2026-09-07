// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Regression tests for reusable local erased borrows.

use qubit_reflect::value::DynamicOwned;
use qubit_reflect::value::DynamicRef;
use qubit_reflect::value::Local;

#[test]
fn local_dynamic_values_can_be_reborrowed_and_cloned() {
    let mut owned = DynamicOwned::<Local>::new(String::from("value"));
    let borrowed = owned.as_reflected_ref();
    let cloned: DynamicRef<'_, Local> = borrowed.clone();

    assert_eq!(borrowed.downcast_ref::<String>().map(String::as_str), Some("value"));
    assert_eq!(cloned.downcast_ref::<String>().map(String::as_str), Some("value"));
    owned
        .as_reflected_mut()
        .downcast_mut::<String>()
        .expect("the owned value must expose an exact mutable borrow")
        .push_str("-updated");
    assert_eq!(
        owned.downcast_ref::<String>().map(String::as_str),
        Some("value-updated")
    );
}
