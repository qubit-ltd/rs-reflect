// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Public contracts for derive-generated type registrations.

#![cfg(feature = "derive")]

use std::any::TypeId;

use qubit_reflect::Reflect;
use qubit_reflect::ReflectionContext;

#[Reflect]
struct DerivedRegistration {
    id: u16,
}

#[test]
fn derived_registration_is_discovered_by_an_explicit_context() {
    let context = ReflectionContext::builder()
        .include_linked_for::<DerivedRegistration>()
        .build()
        .expect("the selected type registration and its dependencies are valid");
    let descriptor = context
        .type_by_name(std::any::type_name::<DerivedRegistration>())
        .expect("the derived type is discoverable in the context");

    assert_eq!(descriptor.type_id(), TypeId::of::<DerivedRegistration>());
    assert!(std::ptr::eq(descriptor.context(), &context));
    assert_eq!(descriptor.field("id").expect("field metadata is retained").name(), "id");
}
