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
use qubit_reflect::TypeDescriptor;
use qubit_reflect::capability::CapabilityRegistration;
use qubit_reflect::capability::CapabilitySpec;
use qubit_reflect::capability::CapabilityTarget;

#[Reflect]
struct ConsumerRecord {
    count: u32,
}

struct ModelCapability;

impl CapabilitySpec for ModelCapability {
    type Adapter = &'static str;

    const ID: &'static str = "public_consumer.model";
}

#[test]
fn public_consumer_keeps_descriptors_and_capabilities_bound_to_their_context() {
    let configured = ReflectionContext::builder()
        .include_type::<ConsumerRecord>()
        .include_type::<u32>()
        .register(CapabilityRegistration::for_type::<ConsumerRecord, ModelCapability>(
            "configured context",
        ))
        .build()
        .expect("the configured consumer context should build");
    let isolated = ReflectionContext::builder()
        .include_type::<ConsumerRecord>()
        .include_type::<u32>()
        .build()
        .expect("the isolated consumer context should build");

    let configured_type = TypeDescriptor::of_in::<ConsumerRecord>(&configured);
    assert!(std::ptr::eq(configured_type.context(), &configured));

    let count = configured_type.field("count").expect("count is reflected");
    let type_expression = count.type_expression();
    assert_eq!(type_expression.display().to_string(), "u32");
    assert!(std::ptr::eq(type_expression.context(), &configured));
    let concrete_type = count.find_concrete_type().expect("u32 is included");
    assert_eq!(concrete_type.type_id(), std::any::TypeId::of::<u32>());
    assert!(std::ptr::eq(concrete_type.context(), &configured));

    let capability = configured_type
        .capability::<ModelCapability>()
        .expect("the configured context has the typed capability");
    assert_eq!(capability.adapter(), &"configured context");
    assert!(matches!(
        capability.target(),
        CapabilityTarget::Type(target) if std::ptr::eq(target.context(), &configured)
    ));

    let isolated_type = TypeDescriptor::of_in::<ConsumerRecord>(&isolated);
    assert!(std::ptr::eq(isolated_type.context(), &isolated));
    assert!(
        isolated_type
            .find_capability::<ModelCapability>()
            .expect("capability lookup succeeds in the isolated context")
            .is_none()
    );
}
