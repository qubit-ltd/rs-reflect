// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

#![cfg(feature = "derive")]

use qubit_reflect::Reflect;
use qubit_reflect::ReflectionContext;
use qubit_reflect::capability::CapabilityOrigin;
use qubit_reflect::capability::CapabilityRegistration;
use qubit_reflect::capability::CapabilitySpec;
use qubit_reflect::capability::CapabilityTarget;
use qubit_reflect::descriptor::TypeDescriptor;

#[Reflect]
struct ConsumerModel {
    count: u32,
}

#[Reflect]
struct GenericConsumerModel<T>(T);

struct ModelCapability;

impl CapabilitySpec for ModelCapability {
    type Adapter = &'static str;

    const ID: &'static str = "external_consumer.model";
}

struct WrongModelCapability;

impl CapabilitySpec for WrongModelCapability {
    type Adapter = u32;

    const ID: &'static str = ModelCapability::ID;
}

#[test]
fn external_consumer_can_use_context_bound_model_and_capability_contract() {
    let first = ReflectionContext::builder()
        .include_type::<ConsumerModel>()
        .include_type::<u32>()
        .register(CapabilityRegistration::for_type::<
            ConsumerModel,
            ModelCapability,
        >("first"))
        .build()
        .expect("the first context includes the model and capability");
    let second = ReflectionContext::builder()
        .include_type::<ConsumerModel>()
        .include_type::<u32>()
        .build()
        .expect("the second context includes the model without the capability");

    let first_type = TypeDescriptor::of_in::<ConsumerModel>(&first);
    let field = first_type.field("count").expect("the field is reflected");
    let type_expression = field.type_expression();
    assert_eq!(type_expression.display().to_string(), "u32");
    assert!(std::ptr::eq(type_expression.context(), &first));
    let concrete_type = field.find_concrete_type().unwrap();
    assert_eq!(concrete_type.type_id(), std::any::TypeId::of::<u32>());
    assert!(std::ptr::eq(concrete_type.context(), &first));
    assert!(std::ptr::eq(field.context(), &first));

    let capability = first_type
        .capability::<ModelCapability>()
        .expect("capability is registered");
    assert_eq!(capability.adapter(), &"first");
    assert!(matches!(
        capability.origin(),
        CapabilityOrigin::Explicit { .. }
    ));
    assert!(matches!(capability.target(), CapabilityTarget::Type(target)
        if std::ptr::eq(target.context(), &first)));

    let second_type = TypeDescriptor::of_in::<ConsumerModel>(&second);
    assert!(std::ptr::eq(second_type.context(), &second));
    assert!(
        second_type
            .find_capability::<ModelCapability>()
            .unwrap()
            .is_none()
    );
}

#[test]
fn external_consumer_can_register_capability_for_generic_definition() {
    let source = ReflectionContext::builder()
        .include_type::<GenericConsumerModel<u8>>()
        .build()
        .expect("generic model can be reflected");
    let definition_id = source
        .type_of::<GenericConsumerModel<u8>>()
        .definition()
        .expect("generic application has a source definition")
        .id();

    let context = ReflectionContext::builder()
        .include_type::<GenericConsumerModel<u8>>()
        .register(CapabilityRegistration::for_definition::<ModelCapability>(
            definition_id,
            "generic",
        ))
        .build()
        .expect("definition capability registration builds");
    let definition = context
        .type_of::<GenericConsumerModel<u8>>()
        .definition()
        .expect("generic application has a context-bound definition");
    let capability = definition
        .capability::<ModelCapability>()
        .expect("definition capability is available");

    assert_eq!(capability.adapter(), &"generic");
    assert!(
        matches!(capability.target(), CapabilityTarget::Definition(target)
        if target.id() == definition.id() && std::ptr::eq(target.context(), &context))
    );
    assert!(matches!(
        capability.origin(),
        CapabilityOrigin::Explicit { .. }
    ));
}

#[test]
fn external_consumer_reports_wrong_adapter_and_keeps_global_context_isolated() {
    let explicit = ReflectionContext::builder()
        .include_type::<ConsumerModel>()
        .register(CapabilityRegistration::for_type::<
            ConsumerModel,
            WrongModelCapability,
        >(42))
        .build()
        .expect("the registered adapter is valid for its own capability spec");
    let explicit_type = explicit.type_of::<ConsumerModel>();
    assert!(matches!(
        explicit_type.capability::<ModelCapability>(),
        Err(qubit_reflect::capability::CapabilityError::ContractMismatch { .. })
    ));

    let global = ReflectionContext::global();
    let global_type = TypeDescriptor::of_in::<ConsumerModel>(global);
    assert!(std::ptr::eq(global_type.context(), global));
    assert!(
        global_type
            .find_capability::<ModelCapability>()
            .unwrap()
            .is_none()
    );
    assert!(
        explicit_type
            .find_capability::<WrongModelCapability>()
            .unwrap()
            .is_some()
    );
}
