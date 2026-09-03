// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Public capability registration conflict integration tests.

use qubit_reflect::error::RegistryErrorKind;
use qubit_reflect::register_type_capabilities;
use qubit_reflect::registry::ReflectRegistry;

#[derive(Clone)]
struct DuplicateCapability;

register_type_capabilities!(DuplicateCapability: Clone);
register_type_capabilities!(DuplicateCapability: Clone);

/// Verifies duplicate public capability registrations fail central registry
/// initialization.
#[test]
fn test_duplicate_public_capability_registration_returns_registry_error() {
    let error = ReflectRegistry::initialize()
        .expect_err("duplicate public capability registrations must invalidate the registry");

    assert_eq!(error.kind(), RegistryErrorKind::CapabilityConflict);
}
