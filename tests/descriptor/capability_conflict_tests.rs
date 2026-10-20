// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Context-level conflict diagnostics for duplicate typed capability providers.

use qubit_reflect::Reflect;
use qubit_reflect::ReflectionContext;
use qubit_reflect::capability::CapabilitySpec;
use qubit_reflect::error::ContextIssue;
use qubit_reflect::registration::CapabilityRegistration;

#[Reflect]
struct DuplicateTarget;

struct DuplicateCapability;

impl CapabilitySpec for DuplicateCapability {
    type Adapter = u32;

    const ID: &'static str = "example.reflect.duplicate";
}

#[test]
fn duplicate_typed_providers_return_a_structured_context_issue() {
    let result = ReflectionContext::builder()
        .include_type::<DuplicateTarget>()
        .register(CapabilityRegistration::for_type::<DuplicateTarget, DuplicateCapability>(3))
        .register(CapabilityRegistration::for_type::<DuplicateTarget, DuplicateCapability>(7))
        .build();
    let error = match result {
        Err(error) => error,
        Ok(_) => panic!("duplicate typed providers must invalidate their context"),
    };

    assert!(matches!(
        error.primary_issue(),
        ContextIssue::ConflictingCapability {
            registrations,
            ..
        } if registrations.len() == 2
    ));
    assert_eq!(error.issues().len(), 1);
}
