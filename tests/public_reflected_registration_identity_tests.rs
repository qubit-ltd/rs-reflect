// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Tests that context registrations preserve exact Rust type identity.

#![cfg(feature = "derive")]

use std::any::TypeId;

use qubit_reflect::{Reflect, ReflectionContext, TypeDescriptor};

#[Reflect]
struct IdentityFixture;

#[test]
fn context_identity_matches_the_registered_rust_type() {
    let context = ReflectionContext::builder()
        .include_type::<IdentityFixture>()
        .build()
        .expect("the fixture registration is valid");
    let registered = context
        .type_by_name(std::any::type_name::<IdentityFixture>())
        .expect("the registered type is indexed");
    let static_descriptor = TypeDescriptor::of::<IdentityFixture>();

    assert_eq!(registered.type_id(), TypeId::of::<IdentityFixture>());
    assert_eq!(static_descriptor.type_id(), TypeId::of::<IdentityFixture>());
    assert!(std::ptr::eq(registered.context(), &context));
    assert!(!std::ptr::eq(
        registered.context(),
        static_descriptor.context()
    ));
}
