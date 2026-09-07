// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Regression coverage for invalid derived capability sets.

use qubit_reflect::__private::testing::build_registry;
use qubit_reflect::Reflect;
use qubit_reflect::capability::CapabilityConflict;
use qubit_reflect::capability::CapabilityConflictKind;
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

#[cfg_attr(feature = "derive", derive(Reflect))]
#[cfg_attr(feature = "derive", reflect(capabilities(first_provider, second_provider)))]
struct IntrinsicConflict;

// Runtime-only builds exercise the same invalid set through handwritten
// registration.
#[cfg(not(feature = "derive"))]
mod runtime_only {
    use std::sync::OnceLock;

    use qubit_reflect::__private::codegen_v3::descriptor::opaque_root;
    use qubit_reflect::Reflect;
    use qubit_reflect::TypeDescriptor;
    use qubit_reflect::capability::CapabilityConflict;
    use qubit_reflect::capability::TypeCapabilities;
    use qubit_reflect::register_reflected_type;

    use super::IntrinsicConflict;
    use super::first_provider;
    use super::second_provider;

    fn capabilities() -> Result<&'static TypeCapabilities, CapabilityConflict> {
        static SET: OnceLock<Result<TypeCapabilities, CapabilityConflict>> = OnceLock::new();
        SET.get_or_init(|| {
            TypeCapabilities::try_new(vec![
                first_provider::<IntrinsicConflict>(),
                second_provider::<IntrinsicConflict>(),
            ])
        })
        .as_ref()
        .map_err(Clone::clone)
    }

    impl Reflect for IntrinsicConflict {
        fn type_descriptor() -> &'static TypeDescriptor {
            static DESCRIPTOR: TypeDescriptor =
                opaque_root::<IntrinsicConflict>("IntrinsicConflict").with_capabilities(capabilities);
            &DESCRIPTOR
        }
    }

    register_reflected_type!(IntrinsicConflict);
}

/// Conflicting generated facts must fail registry construction without
/// panicking.
#[test]
fn test_intrinsic_capability_conflict_is_a_registry_error() {
    let result = std::panic::catch_unwind(ReflectRegistry::initialize);
    let error = result
        .expect("intrinsic capability conflicts must not panic")
        .expect_err("conflicting intrinsic capability IDs must fail registry construction");

    assert_eq!(error.kind(), RegistryErrorKind::CapabilityConflict);
    let conflict = error.intrinsic_conflict().expect("complete conflict retained");
    assert_eq!(conflict.kind(), CapabilityConflictKind::DuplicateId);
    assert!(
        std::error::Error::source(&error)
            .unwrap()
            .downcast_ref::<CapabilityConflict>()
            .is_some()
    );
    let isolated = build_registry(&[]).unwrap();
    assert_eq!(
        isolated.capabilities(IntrinsicConflict::type_descriptor()).unwrap_err(),
        *conflict
    );
    assert_eq!(
        error
            .capability_id()
            .expect("intrinsic conflict must retain its capability ID")
            .as_str(),
        "example.intrinsic_conflict"
    );
}
