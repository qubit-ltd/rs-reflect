// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Static shape access stays independent from context discovery indexes.

#![cfg(feature = "derive")]

use qubit_reflect::{Reflect, ReflectionContext, TypeDescriptor};

#[Reflect]
struct ContextIndependentShape {
    value: u32,
}

#[test]
fn type_metadata_remains_available_when_the_context_does_not_index_the_type() {
    let context = ReflectionContext::builder()
        .build()
        .expect("empty explicit contexts are valid");
    let descriptor = context.type_of::<ContextIndependentShape>();

    assert!(
        context
            .type_by_name(std::any::type_name::<ContextIndependentShape>())
            .is_err()
    );
    assert_eq!(descriptor.context().id(), context.id());
    assert_eq!(
        descriptor
            .field("value")
            .expect("static field metadata is available")
            .name(),
        "value"
    );
    assert_eq!(
        TypeDescriptor::of::<ContextIndependentShape>()
            .field("value")
            .expect("global descriptor also exposes static shape")
            .name(),
        "value"
    );
    assert!(
        context
            .types()
            .expect("empty context stays valid")
            .is_empty()
    );
}
