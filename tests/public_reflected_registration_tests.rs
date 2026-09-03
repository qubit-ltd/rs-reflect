// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Public reflected-type registration integration tests.

use std::any::TypeId;

use qubit_reflect::__private::codegen_v2::descriptor::opaque_root;
use qubit_reflect::descriptor::Reflect;
use qubit_reflect::descriptor::TypeDescriptor;
use qubit_reflect::register_reflected_type;
use qubit_reflect::registry::ReflectRegistry;

struct ManuallyReflected;

static MANUALLY_REFLECTED_DESCRIPTOR: TypeDescriptor = opaque_root::<ManuallyReflected>("ManuallyReflected");

impl Reflect for ManuallyReflected {
    /// Returns the manually authored descriptor used by this integration test.
    fn type_descriptor() -> &'static TypeDescriptor {
        &MANUALLY_REFLECTED_DESCRIPTOR
    }
}

register_reflected_type!(ManuallyReflected);

/// Verifies the public registration macro contributes to the central registry.
#[test]
fn test_register_reflected_type_adds_type_to_reflect_registry() {
    let registry = ReflectRegistry::initialize().expect("the registration must produce a valid registry");
    let registered = registry
        .get(TypeId::of::<ManuallyReflected>())
        .expect("the manually registered type must be discoverable");

    assert!(std::ptr::eq(registered, ManuallyReflected::type_descriptor()));
}
