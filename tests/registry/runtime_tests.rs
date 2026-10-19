// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Integration tests for context-bound type registration and lookup.

#![cfg(feature = "derive")]

use std::any::{TypeId, type_name};

use qubit_reflect::{Reflect, ReflectionContext};

mod duplicate_a {
    use qubit_reflect::Reflect;

    #[Reflect]
    pub struct Shared;
}

mod duplicate_b {
    use qubit_reflect::Reflect;

    #[Reflect]
    pub struct Shared;
}

#[Reflect]
struct ExplicitOnly {
    value: u8,
}

#[test]
fn explicit_context_indexes_only_included_types() {
    let context = ReflectionContext::builder()
        .include_type::<ExplicitOnly>()
        .build()
        .expect("a single type registration is valid");

    assert_eq!(
        context
            .type_by_name(type_name::<ExplicitOnly>())
            .expect("the included type is indexed")
            .type_id(),
        TypeId::of::<ExplicitOnly>()
    );
    assert!(
        context
            .type_by_name(type_name::<duplicate_a::Shared>())
            .is_err()
    );
    assert_eq!(context.types().expect("context remains readable").len(), 1);
}

#[test]
fn descriptor_created_in_context_keeps_that_context() {
    let context = ReflectionContext::builder()
        .include_type::<ExplicitOnly>()
        .build()
        .expect("explicit context builds");
    let descriptor = context.type_of::<ExplicitOnly>();

    assert_eq!(descriptor.context().id(), context.id());
    assert_eq!(descriptor.type_id(), TypeId::of::<ExplicitOnly>());
    assert_eq!(
        descriptor
            .field("value")
            .expect("field metadata is present")
            .name(),
        "value"
    );
}

#[test]
fn short_name_lookup_reports_ambiguity_without_selecting_a_candidate() {
    let context = ReflectionContext::builder()
        .include_type::<duplicate_a::Shared>()
        .include_type::<duplicate_b::Shared>()
        .build()
        .expect("distinct fully qualified type identities may share a short name");

    assert!(context.type_by_short_name("Shared").is_err());
    assert_eq!(
        context
            .type_by_name(type_name::<duplicate_a::Shared>())
            .expect("qualified path identifies the first type")
            .type_id(),
        TypeId::of::<duplicate_a::Shared>()
    );
    assert_eq!(
        context
            .type_by_name(type_name::<duplicate_b::Shared>())
            .expect("qualified path identifies the second type")
            .type_id(),
        TypeId::of::<duplicate_b::Shared>()
    );
}

#[test]
fn global_context_initializes_linked_registrations_once() {
    let first = ReflectionContext::global();
    let initialized = first.initialize().expect("linked registrations are valid");
    let second = ReflectionContext::global();

    assert!(std::ptr::eq(first, second));
    assert!(std::ptr::eq(first, initialized));
    assert!(first.is_initialized());
    assert_eq!(
        first
            .type_of::<ExplicitOnly>()
            .field("value")
            .expect("global context keeps linked type metadata")
            .name(),
        "value"
    );
}
