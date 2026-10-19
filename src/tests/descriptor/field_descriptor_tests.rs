// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0 (the "License");
//    you may not use this file except in compliance with the License.
// =============================================================================

//! Unit coverage for vNext field descriptors and visibility checks.

use crate::runtime::descriptor::TypeDescriptor;
use crate::runtime::descriptor::FieldAccessError;

#[cfg(feature = "derive")]
#[crate::Reflect]
struct PrivateField {
    value: u32,
}

#[test]
#[cfg(feature = "derive")]
#[allow(unsafe_code)]
fn test_private_field_requires_explicit_unsafe_access() {
    let field = TypeDescriptor::of::<PrivateField>()
        .field("value")
        .expect("private field metadata is available");
    let mut target = PrivateField { value: 42 };

    assert!(matches!(field.get::<_, u32>(&target), Err(FieldAccessError::NotPublic)));

    // SAFETY: this test owns the value and explicitly checks bypass behavior.
    assert_eq!(*unsafe { field.get_unsafe::<_, u32>(&target) }.unwrap(), 42);
    // SAFETY: this test owns the value and explicitly checks bypass behavior.
    *unsafe { field.get_mut_unsafe::<_, u32>(&mut target) }.unwrap() = 43;
    assert_eq!(target.value, 43);
}
