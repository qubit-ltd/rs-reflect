// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Public contracts for generic definitions discovered by a context.

#![cfg(feature = "derive")]

use qubit_reflect::{Reflect, ReflectionContext, TypeDescriptor};

#[Reflect]
struct GenericContextRecord<T> {
    value: T,
}

#[test]
fn generic_definition_is_available_from_the_concrete_type_descriptor() {
    let context = ReflectionContext::builder()
        .include_type::<GenericContextRecord<u32>>()
        .build()
        .expect("a concrete generic type registration is valid");
    let concrete = context.type_of::<GenericContextRecord<u32>>();
    let definition = concrete
        .definition()
        .expect("generic type has a definition");

    assert_eq!(definition.name(), "GenericContextRecord");
    assert!(definition.path().ends_with("GenericContextRecord"));
    assert_eq!(concrete.generic_arguments().len(), 1);
    assert_eq!(
        concrete
            .field("value")
            .expect("field metadata is retained")
            .name(),
        "value"
    );

    let found = context
        .type_definition_by_name(definition.path())
        .expect("the context indexes the source definition");
    assert_eq!(found.id(), definition.id());
    assert!(std::ptr::eq(found.context(), &context));
    assert_eq!(
        TypeDescriptor::of::<GenericContextRecord<u32>>()
            .definition()
            .expect("static descriptor also retains the definition")
            .id(),
        definition.id()
    );
}
