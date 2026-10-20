// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Isolated-process coverage for the immutable global reflection context.

#![cfg(feature = "derive")]

use std::any::TypeId;

use qubit_reflect::Reflect;
use qubit_reflect::ReflectionContext;
use qubit_reflect::TypeDescriptor;

#[Reflect]
struct ContextFreezeFixture {
    value: u8,
}

#[test]
fn initialization_discovers_linked_builtins_and_derived_types() {
    let context = ReflectionContext::global()
        .initialize()
        .expect("all static registrations must validate");
    let types = context.types().expect("global context exposes its frozen types");
    assert!(types.iter().any(|ty| ty.type_id() == TypeId::of::<u8>()));
    assert!(
        types
            .iter()
            .any(|ty| ty.type_id() == TypeId::of::<ContextFreezeFixture>())
    );
    assert!(types.iter().all(|ty| std::ptr::eq(ty.context(), context)));
}

#[test]
fn descriptor_queries_do_not_change_the_frozen_context_snapshot() {
    let context = ReflectionContext::global()
        .initialize()
        .expect("all static registrations must validate");
    let before: Vec<_> = context
        .types()
        .expect("context types are available")
        .iter()
        .map(|ty| ty.type_id())
        .collect();

    let descriptor = TypeDescriptor::of::<ContextFreezeFixture>();
    assert!(std::ptr::eq(descriptor.context(), context));
    assert_eq!(
        descriptor.field("value").expect("field metadata exists").name(),
        "value"
    );
    assert_eq!(
        context
            .types()
            .expect("context remains readable")
            .iter()
            .map(|ty| ty.type_id())
            .collect::<Vec<_>>(),
        before
    );
}
