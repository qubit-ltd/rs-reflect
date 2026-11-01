// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![cfg(feature = "derive")]

use qubit_reflect::Reflect;
use qubit_reflect::ReflectionContext;
use qubit_reflect::capability::CapabilitySpec;
use qubit_reflect::registration::Registration;

struct InvalidCapability;

impl CapabilitySpec for InvalidCapability {
    type Adapter = ();

    const ID: &'static str = "invalid..capability";
}

#[Reflect]
struct LinkedCapabilityTarget<T> {
    value: T,
}

fn invalid_linked_capability() -> Registration {
    let capability = __qubit_reflect_v4_definition_capability_LinkedCapabilityTarget::<InvalidCapability>(());
    capability.into()
}

qubit_reflect::__private::codegen_v4::registration::submit_linked_definition_capability!(
    "invalid-linked-capability",
    invalid_linked_capability
);

fn assert_invalid_id(error: qubit_reflect::error::ContextError) {
    assert!(error.to_string().contains("invalid capability ID"), "{error}");
    assert!(error.to_string().contains("invalid..capability"), "{error}");
}

fn invalid_context(
    result: Result<ReflectionContext, qubit_reflect::error::ContextError>,
) -> qubit_reflect::error::ContextError {
    match result {
        Ok(_) => panic!("invalid capability registrations must fail context build"),
        Err(error) => error,
    }
}

#[test]
fn invalid_explicit_capability_fails_context_build() {
    let capability = __qubit_reflect_v4_definition_capability_LinkedCapabilityTarget::<InvalidCapability>(());
    let result = ReflectionContext::builder()
        .include_type::<LinkedCapabilityTarget<u8>>()
        .register(capability)
        .build();
    assert_invalid_id(invalid_context(result));
}

#[test]
fn invalid_linked_capability_fails_all_linked_build_paths() {
    assert_invalid_id(invalid_context(ReflectionContext::builder().include_linked().build()));
    assert_invalid_id(invalid_context(
        ReflectionContext::builder()
            .include_linked_for::<LinkedCapabilityTarget<u8>>()
            .build(),
    ));
}
