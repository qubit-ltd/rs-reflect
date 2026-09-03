// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Public reflected-type registration conflict integration tests.

use qubit_reflect::__private::codegen_v2::descriptor::opaque_root;
use qubit_reflect::descriptor::Reflect;
use qubit_reflect::descriptor::TypeDescriptor;
use qubit_reflect::error::RegistryErrorKind;
use qubit_reflect::register_reflected_type;
use qubit_reflect::registry::ReflectRegistry;

struct DuplicateReflectedType;

static DUPLICATE_REFLECTED_TYPE_DESCRIPTOR: TypeDescriptor =
    opaque_root::<DuplicateReflectedType>("DuplicateReflectedType");

impl Reflect for DuplicateReflectedType {
    fn type_descriptor() -> &'static TypeDescriptor {
        &DUPLICATE_REFLECTED_TYPE_DESCRIPTOR
    }
}

register_reflected_type!(DuplicateReflectedType);
register_reflected_type!(DuplicateReflectedType);

/// Verifies duplicate public type registrations fail central initialization.
#[test]
fn test_duplicate_public_type_registration_returns_registry_error() {
    let error =
        ReflectRegistry::initialize().expect_err("duplicate public type registrations must invalidate the registry");

    assert_eq!(error.kind(), RegistryErrorKind::IdentityConflict);
}
