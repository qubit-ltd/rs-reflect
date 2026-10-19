// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0 (the "License");
//    you may not use this file except in compliance with the License.
// =============================================================================

//! Integration tests for typed vNext capabilities.

use std::rc::Rc;

use qubit_reflect::Reflect;
use qubit_reflect::ReflectionContext;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::capability::{
    CapabilityRegistration, CapabilitySpec, CapabilityTarget, CloneCapability, DefaultCapability,
};
use qubit_reflect::value::{DynamicRef, Local};
use qubit_reflect::register_capability;

#[derive(Clone, Default, Reflect)]
struct BuiltinTarget {
    value: String,
}

#[derive(Reflect)]
struct ExplicitTarget {
    marker: u8,
}

#[derive(Debug, Eq, PartialEq)]
struct LabelAdapter(&'static str);

struct LabelCapability;

impl CapabilitySpec for LabelCapability {
    type Adapter = LabelAdapter;

    const ID: &'static str = "example.reflect.label";
}

register_capability!(BuiltinTarget: CloneCapability);
register_capability!(BuiltinTarget: DefaultCapability);

/// Verifies linked registrations provide built-in Clone and Default contracts.
#[test]
fn test_builtin_capabilities_use_typed_adapters() {
    let descriptor = TypeDescriptor::of::<BuiltinTarget>();
    let original = BuiltinTarget {
        value: String::from("before"),
    };

    let clone = descriptor.capability::<CloneCapability>().expect("Clone is registered");
    let copied = clone
        .adapter()
        .clone_dynamic(DynamicRef::<Local>::new(&original))
        .expect("the registered Clone adapter accepts the exact root type");
    assert_eq!(copied.downcast_ref::<BuiltinTarget>().unwrap().value, "before");

    let default = descriptor.capability::<DefaultCapability>().expect("Default is registered");
    let created = default.adapter().create_dynamic::<Local>();
    assert_eq!(created.downcast_ref::<BuiltinTarget>().unwrap().value, "");
}

/// Verifies a custom capability is attached to an explicit context and its
/// adapter remains type checked by the specification.
#[test]
fn test_custom_capability_is_bound_to_its_explicit_context() {
    let context = ReflectionContext::builder()
        .include_type::<ExplicitTarget>()
        .register(CapabilityRegistration::for_type::<ExplicitTarget, LabelCapability>(
            LabelAdapter("explicit"),
        ))
        .build()
        .expect("the explicit capability registration is valid");
    let descriptor = context.type_of::<ExplicitTarget>();
    let capability = descriptor.capability::<LabelCapability>().expect("custom capability exists");

    assert_eq!(capability.adapter(), &LabelAdapter("explicit"));
    assert!(matches!(capability.target(), CapabilityTarget::Type(target) if target.type_id() == descriptor.type_id()));
    assert!(std::ptr::eq(descriptor.context(), &context));
    assert_eq!(descriptor.capabilities().unwrap().iter().len(), 1);
    let value = ExplicitTarget { marker: 1 };
    assert_eq!(value.marker, 1);
}

/// Verifies a local-only target can still expose a statically typed custom
/// capability contract.
#[test]
fn test_local_target_can_use_a_typed_capability_spec() {
    #[derive(Reflect)]
    struct LocalOnly {
        value: Rc<()>,
    }

    let context = ReflectionContext::builder()
        .include_type::<LocalOnly>()
        .register(CapabilityRegistration::for_type::<LocalOnly, LabelCapability>(LabelAdapter("local")))
        .build()
        .expect("a local-only type can have a typed registration");
    let local = LocalOnly { value: Rc::new(()) };
    let descriptor = context.type_of::<LocalOnly>();
    let capability = descriptor.capability::<LabelCapability>().expect("custom capability exists");
    assert_eq!(descriptor.type_id(), std::any::TypeId::of::<LocalOnly>());
    assert_eq!(capability.adapter(), &LabelAdapter("local"));
    assert_eq!(Rc::strong_count(&local.value), 1);
}
