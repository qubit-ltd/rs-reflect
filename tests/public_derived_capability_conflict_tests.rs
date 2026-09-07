// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Derived and explicit capability conflict integration tests.

use qubit_reflect::Reflect;
use qubit_reflect::error::RegistryErrorKind;
use qubit_reflect::register_type_capabilities;
use qubit_reflect::registry::ReflectRegistry;

#[derive(Clone, Reflect)]
#[reflect(capabilities(Clone))]
struct DuplicateDerivedCapability;

register_type_capabilities!(DuplicateDerivedCapability: Clone);

/// Verifies derived and explicit facts share one conflict authority.
#[test]
fn test_derived_and_explicit_capability_conflict_returns_registry_error() {
    let error =
        ReflectRegistry::initialize().expect_err("duplicate derived and explicit facts must invalidate the registry");

    assert_eq!(error.kind(), RegistryErrorKind::CapabilityConflict);
}
