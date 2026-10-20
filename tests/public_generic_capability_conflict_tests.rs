// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![cfg(feature = "derive")]

use std::any::TypeId;

use qubit_reflect::Reflect;
use qubit_reflect::ReflectionContext;
use qubit_reflect::capability::CapabilitySpec;
use qubit_reflect::capability::CapabilityTargetId;
use qubit_reflect::error::ContextIssue;
use qubit_reflect::registration::CapabilityRegistration;

#[Reflect]
struct GenericTarget<T>(T);

struct FirstCapability;

impl CapabilitySpec for FirstCapability {
    type Adapter = u32;

    const ID: &'static str = "example.generic_conflict";
}

struct SecondCapability;

impl CapabilitySpec for SecondCapability {
    type Adapter = String;

    const ID: &'static str = FirstCapability::ID;
}

#[test]
fn conflicting_registration_reports_the_concrete_generic_target() {
    let error = match ReflectionContext::builder()
        .include_type::<GenericTarget<u32>>()
        .register(CapabilityRegistration::for_type::<GenericTarget<u32>, FirstCapability>(
            1,
        ))
        .register(
            CapabilityRegistration::for_type::<GenericTarget<u32>, SecondCapability>(String::from("second adapter")),
        )
        .build()
    {
        Ok(_) => panic!("conflicting capability contracts for one monomorph must be rejected"),
        Err(error) => error,
    };

    assert!(
        matches!(
            error.primary_issue(),
            ContextIssue::ConflictingCapability { target, .. }
                if matches!(target, CapabilityTargetId::Type(identity)
                    if identity.type_id == TypeId::of::<GenericTarget<u32>>())
        ),
        "unexpected issue: {:?}",
        error.primary_issue()
    );
    assert_eq!(error.issues().len(), 1);

    let independent_context = ReflectionContext::builder()
        .include_type::<GenericTarget<String>>()
        .register(CapabilityRegistration::for_type::<GenericTarget<String>, FirstCapability>(2))
        .build()
        .expect("a capability on a different monomorph has no conflict");
    assert_eq!(
        *independent_context
            .type_of::<GenericTarget<String>>()
            .capability::<FirstCapability>()
            .expect("the generic monomorph has its registered capability")
            .adapter(),
        2
    );
}

#[test]
fn conflicting_definition_registrations_preserve_identity_and_origins() {
    let definition_id = ReflectionContext::builder()
        .include_type::<GenericTarget<u32>>()
        .build()
        .expect("the generic definition context is valid")
        .type_of::<GenericTarget<u32>>()
        .definition()
        .expect("the generic target has a definition")
        .id();

    let error = match ReflectionContext::builder()
        .include_type::<GenericTarget<u32>>()
        .register(CapabilityRegistration::for_definition::<FirstCapability>(
            definition_id,
            1,
        ))
        .register(CapabilityRegistration::for_definition::<SecondCapability>(
            definition_id,
            String::from("second"),
        ))
        .build()
    {
        Ok(_) => {
            panic!("conflicting definition capability providers must invalidate their context")
        }
        Err(error) => error,
    };

    assert!(matches!(
        error.primary_issue(),
        ContextIssue::ConflictingCapability {
            target: CapabilityTargetId::Definition(identity),
            registrations,
            ..
        } if *identity == definition_id
            && registrations.len() == 2
    ));
    assert_eq!(error.issues().len(), 1);
}
