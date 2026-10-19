// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Public capability registration conflict integration tests.

#![cfg(feature = "derive")]

use qubit_reflect::{Reflect, ReflectionContext};
use qubit_reflect::capability::{CloneAdapter, CloneCapability};
use qubit_reflect::registration::CapabilityRegistration;
use qubit_reflect::error::ContextIssue;

#[Reflect]
#[derive(Clone)]
struct DuplicateCapabilityTarget;

fn clone_registration() -> CapabilityRegistration {
    CapabilityRegistration::for_type::<DuplicateCapabilityTarget, CloneCapability>(
        CloneAdapter::for_type::<DuplicateCapabilityTarget>(),
    )
}

#[test]
fn duplicate_public_capability_registration_returns_context_conflict() {
    let result = ReflectionContext::builder()
        .include_type::<DuplicateCapabilityTarget>()
        .register(clone_registration())
        .register(clone_registration())
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
