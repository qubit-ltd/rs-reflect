// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Public diagnostics for conflicting typed registrations in one context.

#![cfg(feature = "derive")]

use qubit_reflect::Reflect;
use qubit_reflect::ReflectionContext;
use qubit_reflect::capability::CapabilitySpec;
use qubit_reflect::error::ContextIssue;
use qubit_reflect::registration::CapabilityRegistration;

#[Reflect]
#[derive(Clone)]
struct ConflictingCloneTarget;

struct U32AdapterSpec;
struct U64AdapterSpec;

impl CapabilitySpec for U32AdapterSpec {
    type Adapter = u32;
    const ID: &'static str = "example.context.adapter_conflict";
}

impl CapabilitySpec for U64AdapterSpec {
    type Adapter = u64;
    const ID: &'static str = "example.context.adapter_conflict";
}

#[test]
fn duplicate_typed_providers_report_a_structured_context_conflict() {
    let result = ReflectionContext::builder()
        .include_type::<ConflictingCloneTarget>()
        .register(CapabilityRegistration::for_type::<ConflictingCloneTarget, U32AdapterSpec>(3_u32))
        .register(CapabilityRegistration::for_type::<ConflictingCloneTarget, U64AdapterSpec>(7_u64))
        .build();
    let error = match result {
        Err(error) => error,
        Ok(_) => panic!("a context cannot choose between duplicate capability providers"),
    };

    assert!(matches!(
        error.primary_issue(),
        ContextIssue::ConflictingCapability { .. }
    ));
    assert_eq!(error.issues().len(), 1);
}
