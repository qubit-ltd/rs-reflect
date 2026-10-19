// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Public contracts for isolated, explicitly built reflection contexts.

#![cfg(feature = "derive")]

use qubit_reflect::Reflect;
use qubit_reflect::ReflectionContext;
use qubit_reflect::capability::{CapabilityRegistration, CloneAdapter, CloneCapability};

#[Reflect]
#[derive(Clone, Debug, Eq, PartialEq)]
struct ExplicitCloneTarget(String);

#[Reflect]
struct UnregisteredTarget;

#[test]
fn empty_context_does_not_import_linked_registrations() {
    let context = ReflectionContext::builder()
        .build()
        .expect("an empty explicit context is valid");

    assert!(
        context
            .types()
            .expect("empty context can be queried")
            .is_empty()
    );
    assert!(
        context
            .type_by_name(std::any::type_name::<ExplicitCloneTarget>())
            .is_err()
    );
}

#[test]
fn explicit_capabilities_are_isolated_from_the_global_context() {
    let registration = CapabilityRegistration::for_type::<ExplicitCloneTarget, CloneCapability>(
        CloneAdapter::for_type::<ExplicitCloneTarget>(),
    );
    let context = ReflectionContext::builder()
        .include_type::<ExplicitCloneTarget>()
        .register(registration)
        .build()
        .expect("one typed capability registration is valid");

    assert!(
        context
            .type_of::<ExplicitCloneTarget>()
            .capability::<CloneCapability>()
            .is_ok()
    );
    assert!(
        ReflectionContext::builder()
            .include_type::<ExplicitCloneTarget>()
            .build()
            .expect("a context without extension registrations is valid")
            .type_of::<ExplicitCloneTarget>()
            .find_capability::<CloneCapability>()
            .expect("the absence is a valid query")
            .is_none()
    );
}

#[test]
fn explicit_type_members_do_not_implicitly_register_other_types() {
    let context = ReflectionContext::builder()
        .include_type::<ExplicitCloneTarget>()
        .build()
        .expect("the selected type can be registered by itself");

    assert_eq!(
        context.types().expect("context types are available").len(),
        1
    );
    assert!(
        context
            .type_by_name(std::any::type_name::<UnregisteredTarget>())
            .is_err()
    );
    assert_eq!(
        context.type_of::<ExplicitCloneTarget>().context().id(),
        context.id()
    );
}

#[test]
fn conflicting_capability_registrations_fail_context_construction() {
    let registration = || {
        CapabilityRegistration::for_type::<ExplicitCloneTarget, CloneCapability>(
            CloneAdapter::for_type::<ExplicitCloneTarget>(),
        )
    };
    let result = ReflectionContext::builder()
        .include_type::<ExplicitCloneTarget>()
        .register(registration())
        .register(registration())
        .build();

    assert!(
        result.is_err(),
        "duplicate typed providers must be rejected"
    );
}
