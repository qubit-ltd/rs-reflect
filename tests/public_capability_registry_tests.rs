// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Public capability projection integration tests.

use std::any::TypeId;

use qubit_reflect::__private::codegen_v2::descriptor::opaque_root;
use qubit_reflect::capability::clone_key;
use qubit_reflect::descriptor::Reflect;
use qubit_reflect::descriptor::TypeDescriptor;
use qubit_reflect::register_reflected_type;
use qubit_reflect::register_type_capabilities;
use qubit_reflect::registry::ReflectRegistry;

#[derive(Clone)]
struct RegisteredCapability;

static REGISTERED_CAPABILITY_DESCRIPTOR: TypeDescriptor = opaque_root::<RegisteredCapability>("RegisteredCapability");

impl Reflect for RegisteredCapability {
    fn type_descriptor() -> &'static TypeDescriptor {
        &REGISTERED_CAPABILITY_DESCRIPTOR
    }
}

register_reflected_type!(RegisteredCapability);
register_type_capabilities!(RegisteredCapability: Clone);

/// Verifies capabilities are projected from the central registry snapshot.
#[test]
fn test_registry_projects_typed_capabilities() {
    let registry = ReflectRegistry::initialize().expect("the registrations must be valid");
    let capabilities = registry.capabilities(RegisteredCapability::type_descriptor());

    assert!(capabilities.contains(clone_key()));
    assert!(
        registry
            .capability_by_id(RegisteredCapability::type_descriptor(), "qubit.reflect.clone",)
            .is_some()
    );
    assert!(
        registry
            .capability_by_id(RegisteredCapability::type_descriptor(), "not-valid!")
            .is_none()
    );
    let matches: Vec<_> = registry.types_with_capability(clone_key()).collect();
    assert!(
        matches
            .iter()
            .any(|descriptor| std::ptr::eq(*descriptor, RegisteredCapability::type_descriptor()))
    );
    assert!(registry.type_source(TypeId::of::<RegisteredCapability>()).is_some());
    assert_eq!(registry.types_with_identity().count(), registry.types().len());
}
