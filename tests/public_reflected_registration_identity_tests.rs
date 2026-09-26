// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::descriptor::Reflect;
use qubit_reflect::descriptor::TypeDescriptor;
use qubit_reflect::error::RegistryErrorKind;
use qubit_reflect::register_reflected_type;
use qubit_reflect::registry::ReflectRegistry;

struct Wrong;
impl Reflect for Wrong {
    fn type_descriptor() -> &'static TypeDescriptor {
        TypeDescriptor::of::<u32>()
    }
}
register_reflected_type!(Wrong);

#[test]
fn wrong_registered_descriptor_returns_structured_identity_conflict() {
    let error = ReflectRegistry::initialize().expect_err("wrong descriptor identity must fail registration");
    assert_eq!(error.kind(), RegistryErrorKind::IdentityConflict);
    assert!(error.conflicting_fragments().is_some());
}
