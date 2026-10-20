// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::Reflect;
use qubit_reflect::ReflectionContext;
use qubit_reflect::descriptor::TypeDescriptor;

struct NotReflected;

#[Reflect]
pub struct Record {
    pub visible: u32,
    #[reflect(skip)]
    // This member remains a Rust field but must not enter reflection metadata.
    #[allow(dead_code)]
    ignored: NotReflected,
}

#[test]
fn skipped_member_is_absent_from_the_descriptor() {
    let descriptor = TypeDescriptor::of_in::<Record>(ReflectionContext::global());
    let record = Record {
        visible: 5,
        ignored: NotReflected,
    };

    assert_eq!(descriptor.fields().len(), 1);
    assert!(descriptor.fields().get("visible").is_some());
    assert!(descriptor.fields().get("ignored").is_none());
    assert!(descriptor.fields().get_at(1).is_none());
    assert_eq!(*descriptor.field("visible").unwrap().get::<_, u32>(&record).unwrap(), 5);
}
