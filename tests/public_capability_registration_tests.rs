// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![cfg(feature = "derive")]

use qubit_reflect::__private::codegen_v4::expression::GenericDefinitionMetadata;
use qubit_reflect::__private::codegen_v4::expression::StructKind;
use qubit_reflect::__private::codegen_v4::expression::TypeDefinitionKind;
use qubit_reflect::__private::codegen_v4::traits::Completeness;
use qubit_reflect::__private::codegen_v4::traits::TraitDefinitionSpec;
use qubit_reflect::__private::codegen_v4::traits::TraitVisibility;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectionContext;
use qubit_reflect::capability::CapabilityError;
use qubit_reflect::capability::CapabilityRegistration;
use qubit_reflect::capability::CapabilitySpec;
use qubit_reflect::capability::CapabilityTarget as CapabilityTargetView;
use qubit_reflect::descriptor::MethodDescriptor;
use qubit_reflect::registration::Registration;
use qubit_reflect::registration::TypeDefinitionRegistration;

#[Reflect]
struct CapabilityTarget;

#[Reflect]
struct MethodCatalogOnlyTarget;

impl MethodCatalogOnlyTarget {
    fn generated_catalog_method(&self) {}
}

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

struct FakeTypeDefinitionMarker;
struct SameNameTraitMarker;

static EMPTY_GENERIC_DEFINITION: GenericDefinitionMetadata = GenericDefinitionMetadata::new(&[], &[]);

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
fn generated_method_catalog_does_not_satisfy_capability_target_inclusion() {
    let registration = CapabilityRegistration::for_type::<MethodCatalogOnlyTarget, MarkerCapability>(7);
    let missing_target = ReflectionContext::builder().register(registration).build();
    let error = match missing_target {
        Ok(_) => panic!("a method catalog does not include its target type"),
        Err(error) => error,
    };
    assert!(
        error
            .issues()
            .iter()
            .any(|issue| matches!(issue, qubit_reflect::error::ContextIssue::MissingDependency { .. }))
    );

    let context = ReflectionContext::builder()
        .include_type::<MethodCatalogOnlyTarget>()
        .register(CapabilityRegistration::for_type::<
            MethodCatalogOnlyTarget,
            MarkerCapability,
        >(7))
        .build()
        .expect("including the reflected target satisfies capability registration dependency");
    MethodCatalogOnlyTarget.generated_catalog_method();
    assert!(
        context
            .type_of::<MethodCatalogOnlyTarget>()
            .has_capability::<MarkerCapability>()
            .unwrap()
    );
}

#[test]
fn capability_registration_accepts_unsized_reflected_types() {
    let registration = CapabilityRegistration::for_type::<str, MarkerCapability>(11);
    let _: Registration = registration.into();
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
        source_context.type_definition_by_id(definition_id).unwrap().id(),
        definition_id
    );
    let registration = CapabilityRegistration::for_definition::<MarkerCapability>(definition_id, 9);
    let context = ReflectionContext::builder()
        .include_type::<GenericCapabilityTarget<u8>>()
        .register(registration)
        .build()
        .expect("definition-targeted capability registration builds");

    let type_descriptor = context.type_of::<GenericCapabilityTarget<u8>>();
    assert!(type_descriptor.is_registered().unwrap());
    let definition = type_descriptor.definition().unwrap();
    let capability = definition.capability::<MarkerCapability>().unwrap();
    assert_eq!(capability.adapter(), &9);
    assert!(definition.has_capability::<MarkerCapability>().unwrap());
    assert!(matches!(capability.target(), CapabilityTargetView::Definition(target)
        if target.id() == definition.id() && std::ptr::eq(target.context(), &context)));
    let capabilities = definition.capabilities().unwrap();
    assert!(matches!(capabilities.target(), CapabilityTargetView::Definition(target)
        if target.id() == definition.id() && std::ptr::eq(target.context(), &context)));
}

#[test]
fn definition_capability_requires_exact_type_definition_identity() {
    let source_context = ReflectionContext::builder()
        .include_type::<GenericCapabilityTarget<u8>>()
        .build()
        .expect("generic type registration builds");
    let definition_id = source_context
        .type_of::<GenericCapabilityTarget<u8>>()
        .definition()
        .expect("the reflected generic application has a definition")
        .id();
    let identity_text = format!("{definition_id:?}");
    let leaked_identity: &'static str = Box::leak(identity_text.into_boxed_str());

    let same_name_type_definition = Box::leak(Box::new(TypeDefinitionRegistration::<FakeTypeDefinitionMarker>::new(
        leaked_identity,
        leaked_identity,
        TypeDefinitionKind::Struct(StructKind::Named),
        &EMPTY_GENERIC_DEFINITION,
    )));
    let mismatched_type = ReflectionContext::builder()
        .register(Registration::type_definition(same_name_type_definition))
        .register(CapabilityRegistration::for_definition::<MarkerCapability>(
            definition_id,
            12,
        ))
        .build();
    assert!(matches!(
        mismatched_type,
        Err(error) if error.issues().iter().any(|issue| matches!(
            issue,
            qubit_reflect::error::ContextIssue::MissingDependency { .. }
        ))
    ));

    let same_name_trait = Box::leak(Box::new(TraitDefinitionSpec::<SameNameTraitMarker>::new(
        leaked_identity,
        leaked_identity,
        TraitVisibility::Public,
        Completeness::Complete,
        &EMPTY_GENERIC_DEFINITION,
        &[],
        &[],
        &[],
        &[],
    )));
    let wrong_kind = ReflectionContext::builder()
        .register(Registration::metadata_only().with_trait_definition(same_name_trait.metadata()))
        .register(CapabilityRegistration::for_definition::<MarkerCapability>(
            definition_id,
            13,
        ))
        .build();
    assert!(matches!(
        wrong_kind,
        Err(error) if error.issues().iter().any(|issue| matches!(
            issue,
            qubit_reflect::error::ContextIssue::MissingDependency { .. }
        ))
    ));
}

#[test]
fn typed_capability_lookup_reports_missing_and_contract_mismatch() {
    let missing_context = ReflectionContext::builder()
        .include_type::<CapabilityTarget>()
        .build()
        .expect("type without capabilities still builds");
    let target = missing_context.type_of::<CapabilityTarget>();
    assert!(target.capabilities().unwrap().is_empty());
    assert!(!target.has_capability::<MarkerCapability>().unwrap());
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
    assert!(!definition.has_capability::<MarkerCapability>().unwrap());
    assert!(definition.find_capability::<MarkerCapability>().unwrap().is_none());
    assert!(matches!(
        definition.capability::<MarkerCapability>(),
        Err(CapabilityError::Missing { capability_id, .. })
            if capability_id == MarkerCapability::ID
    ));

    let registration =
        CapabilityRegistration::for_type::<CapabilityTarget, SameIdWrongContract>(String::from("wrong adapter"));
    let context = ReflectionContext::builder()
        .include_type::<CapabilityTarget>()
        .register(registration)
        .build()
        .expect("a stored adapter contract is checked at typed lookup");
    assert!(matches!(
        context.type_of::<CapabilityTarget>().capability::<MarkerCapability>(),
        Err(CapabilityError::ContractMismatch { .. })
    ));
    assert!(matches!(
        context
            .type_of::<CapabilityTarget>()
            .has_capability::<MarkerCapability>(),
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

    let registration = CapabilityRegistration::for_type::<CapabilityTarget, InvalidIdCapability>(());
    let result = ReflectionContext::builder()
        .include_type::<CapabilityTarget>()
        .register(registration)
        .build();
    let error = match result {
        Ok(_) => panic!("invalid capability ID should be rejected"),
        Err(error) => error,
    };
    assert!(
        error
            .issues()
            .iter()
            .any(|issue| matches!(issue, qubit_reflect::error::ContextIssue::InvalidMetadata { .. }))
    );
}

#[allow(dead_code)]
fn assert_method_implementation_source_is_public(method: &MethodDescriptor<'_>) {
    let _: qubit_reflect::descriptor::MethodImplementationSource = method.implementation_source();
}
