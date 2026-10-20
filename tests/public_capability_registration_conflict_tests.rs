// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Public capability registration conflict integration tests.

#![cfg(feature = "derive")]

use qubit_reflect::Reflect;
use qubit_reflect::ReflectionContext;
use qubit_reflect::error::ContextIssue;
use qubit_reflect::register_capability;

#[Reflect]
#[derive(Clone)]
struct DuplicateCapabilityTarget;

register_capability!(DuplicateCapabilityTarget: Clone);
register_capability!(DuplicateCapabilityTarget: Clone);

#[test]
fn duplicate_public_capability_registration_returns_context_conflict() {
    let result = ReflectionContext::builder()
        .include_linked_for::<DuplicateCapabilityTarget>()
        .build();
    let error = match result {
        Err(error) => error,
        Ok(_) => panic!("duplicate typed capability registrations must be rejected"),
    };

    assert!(matches!(
        error.primary_issue(),
        ContextIssue::ConflictingCapability { .. }
    ));
}
