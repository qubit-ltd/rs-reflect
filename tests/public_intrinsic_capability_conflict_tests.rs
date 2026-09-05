// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

//! Regression coverage for invalid derived capability sets.

use qubit_reflect::Reflect;
use qubit_reflect::capability::CapabilityDescriptor;
use qubit_reflect::capability::CapabilityKey;
use qubit_reflect::error::RegistryErrorKind;
use qubit_reflect::identity::CapabilityId;
use qubit_reflect::registry::ReflectRegistry;

/// Returns the shared ID intentionally claimed by both fixture providers.
fn conflicting_key() -> CapabilityKey<fn()> {
    CapabilityKey::new(CapabilityId::new("example.intrinsic_conflict").expect("fixture ID is valid"))
}

/// Provides the first conflicting capability declaration.
fn first_provider<T: 'static>() -> CapabilityDescriptor {
    let _ = std::marker::PhantomData::<T>;
    CapabilityDescriptor::with_adapter(conflicting_key(), first_adapter as fn())
}

/// Provides the second conflicting capability declaration.
fn second_provider<T: 'static>() -> CapabilityDescriptor {
    let _ = std::marker::PhantomData::<T>;
    CapabilityDescriptor::with_adapter(conflicting_key(), second_adapter as fn())
}

fn first_adapter() {}
fn second_adapter() {}

#[derive(Reflect)]
#[reflect(capabilities(first_provider, second_provider))]
struct IntrinsicConflict;

/// Conflicting generated facts must fail registry construction without
/// panicking.
#[test]
fn test_intrinsic_capability_conflict_is_a_registry_error() {
    let result = std::panic::catch_unwind(ReflectRegistry::initialize);
    let error = result
        .expect("intrinsic capability conflicts must not panic")
        .expect_err("conflicting intrinsic capability IDs must fail registry construction");

    assert_eq!(error.kind(), RegistryErrorKind::CapabilityConflict);
    assert_eq!(
        error
            .capability_id()
            .expect("intrinsic conflict must retain its capability ID")
            .as_str(),
        "example.intrinsic_conflict"
    );
}
