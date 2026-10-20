// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::capability::CapabilitySpec;
use qubit_reflect::context::ReflectionContext;
use qubit_reflect::context::Registration;
use qubit_reflect::context::TraitObjectRegistration;
use qubit_reflect::context::TypeDefinitionRegistration;
use qubit_reflect::descriptor::GenericDefinitionMetadata;
use qubit_reflect::descriptor::Reflect;
use qubit_reflect::descriptor::StructKind;
use qubit_reflect::descriptor::TypeDefinitionKind;
use qubit_reflect::descriptor::TypeRegistration;
use qubit_reflect::registration::CapabilityRegistration;

struct Target;

impl Reflect for Target {}

struct GenericDefinition;

struct TestCapability;

impl CapabilitySpec for TestCapability {
    type Adapter = u8;

    const ID: &'static str = "tests.registration_counter";
}

static GENERICS: GenericDefinitionMetadata = GenericDefinitionMetadata::new(&[], &[]);
static DEFINITION: TypeDefinitionRegistration<GenericDefinition> = TypeDefinitionRegistration::new(
    "GenericDefinition",
    "tests::GenericDefinition",
    TypeDefinitionKind::Struct(StructKind::Unit),
    &GENERICS,
);
static TARGET_REGISTRATION: TypeRegistration<Target> = TypeRegistration::new();

#[test]
fn public_registration_facades_expose_constructible_typed_inputs() {
    let capability = CapabilityRegistration::for_type::<String, TestCapability>(7);

    let context = ReflectionContext::builder()
        .register(&TARGET_REGISTRATION)
        .register(Registration::type_definition(&DEFINITION))
        .register(
            Registration::metadata_only()
                .with_trait_object(TraitObjectRegistration::of::<dyn std::fmt::Debug>("std::fmt::Debug")),
        )
        .register(Registration::of::<String>())
        .register(capability)
        .build()
        .expect("typed public registrations should build a context");

    assert_eq!(context.type_definitions().expect("type definitions").len(), 1);
}
