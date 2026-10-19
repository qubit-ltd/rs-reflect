// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

//! Public contract for explicitly registering typed capability adapters.

#![cfg(feature = "derive")]

use qubit_reflect::capability::{
    CapabilityError, CapabilityRegistration, CapabilitySpec, TypeDefinitionCapabilityExt,
    TypeDescriptorCapabilityExt,
};
use qubit_reflect::descriptor::MethodDescriptor;
use qubit_reflect::registration::Registration;
use qubit_reflect::{Reflect, ReflectionContext};

#[Reflect]
struct CapabilityTarget;

#[Reflect]
struct GenericCapabilityTarget<T>(T);

struct MarkerCapability;

impl CapabilitySpec for MarkerCapability {
    type Adapter = u32;

    const ID: &'static str = "example.public.registration_marker";
}

struct SameIdWrongContract;

impl CapabilitySpec for SameIdWrongContract {
    type Adapter = String;

    const ID: &'static str = MarkerCapability::ID;
}

#[test]
fn explicit_typed_capability_registration_converts_into_context_registration() {
    let registration = CapabilityRegistration::for_type::<CapabilityTarget, MarkerCapability>(7);
    let registration: Registration = registration.into();
    let context = ReflectionContext::builder()
        .include_type::<CapabilityTarget>()
        .register(registration)
        .build()
        .expect("typed capability registration builds in an explicit context");

    assert!(context.type_of::<CapabilityTarget>().is_registered().unwrap());
}

#[test]
fn capability_registration_can_target_a_generic_definition_without_exposing_its_id() {
    let source_context = ReflectionContext::builder()
        .include_type::<GenericCapabilityTarget<u8>>()
        .build()
        .expect("generic type registration builds");
    let definition = source_context
        .type_of::<GenericCapabilityTarget<u8>>()
        .definition()
        .expect("the reflected generic application has a definition");
    let definition_id = definition.id();
    assert_eq!(
        source_context
            .type_definition_by_id(definition_id)
            .unwrap()
            .id(),
        definition_id
    );
    let registration =
        CapabilityRegistration::for_definition::<MarkerCapability>(definition_id, 9);
    let context = ReflectionContext::builder()
        .include_type::<GenericCapabilityTarget<u8>>()
        .register(registration)
        .build()
        .expect("definition-targeted capability registration builds");

    let type_descriptor = context.type_of::<GenericCapabilityTarget<u8>>();
    assert!(type_descriptor.is_registered().unwrap());
    let definition = type_descriptor.definition().unwrap();
    assert_eq!(definition.capability::<MarkerCapability>().unwrap().adapter(), &9);
}

#[test]
fn typed_capability_lookup_reports_missing_and_contract_mismatch() {
    let missing_context = ReflectionContext::builder()
        .include_type::<CapabilityTarget>()
        .build()
        .expect("type without capabilities still builds");
    let target = missing_context.type_of::<CapabilityTarget>();
    assert!(target.capabilities().unwrap().is_empty());
    assert!(target.find_capability::<MarkerCapability>().unwrap().is_none());
    assert!(matches!(
        target.capability::<MarkerCapability>(),
        Err(CapabilityError::Missing { capability_id, .. })
            if capability_id == MarkerCapability::ID
    ));

    let generic_context = ReflectionContext::builder()
        .include_type::<GenericCapabilityTarget<u8>>()
        .build()
        .expect("generic definition without capabilities still builds");
    let definition = generic_context
        .type_of::<GenericCapabilityTarget<u8>>()
        .definition()
        .unwrap();
    assert!(definition.capabilities().unwrap().is_empty());
    assert!(definition
        .find_capability::<MarkerCapability>()
        .unwrap()
        .is_none());
    assert!(matches!(
        definition.capability::<MarkerCapability>(),
        Err(CapabilityError::Missing { capability_id, .. })
            if capability_id == MarkerCapability::ID
    ));

    let registration = CapabilityRegistration::for_type::<CapabilityTarget, SameIdWrongContract>(
        String::from("wrong adapter"),
    );
    let context = ReflectionContext::builder()
        .include_type::<CapabilityTarget>()
        .register(registration)
        .build()
        .expect("a stored adapter contract is checked at typed lookup");
    assert!(matches!(
        context.type_of::<CapabilityTarget>().capability::<MarkerCapability>(),
        Err(CapabilityError::ContractMismatch { .. })
    ));
}

#[test]
fn invalid_capability_id_is_reported_by_context_build_without_panicking() {
    struct InvalidIdCapability;

    impl CapabilitySpec for InvalidIdCapability {
        type Adapter = ();

        const ID: &'static str = "invalid-capability-id";
    }

    let registration =
        CapabilityRegistration::for_type::<CapabilityTarget, InvalidIdCapability>(());
    let result = ReflectionContext::builder()
        .include_type::<CapabilityTarget>()
        .register(registration)
        .build();
    let error = match result {
        Ok(_) => panic!("invalid capability ID should be rejected"),
        Err(error) => error,
    };
    assert!(error.issues().iter().any(|issue| matches!(
        issue,
        qubit_reflect::error::ContextIssue::InvalidMetadata { .. }
    )));
}

#[allow(dead_code)]
fn assert_method_implementation_source_is_public(method: &MethodDescriptor<'_>) {
    let _: qubit_reflect::descriptor::MethodImplementationSource = method.implementation_source();
}
