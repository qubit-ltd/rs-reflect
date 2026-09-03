// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow explicit-imports
//! Isolated-process coverage for statically frozen built-in registry roots.

use std::any::TypeId;

use qubit_reflect as reflect;
use qubit_reflect::__private::codegen_v2::registration::FragmentKind;
use qubit_reflect::__private::codegen_v2::registration::FragmentPayload;
use qubit_reflect::__private::codegen_v2::registration::RegistrationFragment;
use qubit_reflect::__private::codegen_v2::registration::RuntimeIdentity;
use qubit_reflect::__private::codegen_v2::registration::StaticFragmentIdentity;
use qubit_reflect::Reflect;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::registry::ReflectRegistry;

struct ExplicitRegistration;

static EXPLICIT_DESCRIPTOR: TypeDescriptor =
    reflect::__private::codegen_v2::descriptor::opaque_root::<ExplicitRegistration>("explicit_registration");

impl Reflect for ExplicitRegistration {
    /// Returns the exact root submitted by the explicit registration fixture.
    fn type_descriptor() -> &'static TypeDescriptor {
        &EXPLICIT_DESCRIPTOR
    }
}

/// Returns the exact process-local identity of the explicit fixture root.
fn explicit_runtime_identity() -> RuntimeIdentity {
    RuntimeIdentity::Type(TypeId::of::<ExplicitRegistration>())
}

/// Returns the exact root submitted by the explicit fixture fragment.
fn explicit_payload() -> FragmentPayload {
    FragmentPayload::Type(&EXPLICIT_DESCRIPTOR)
}

reflect::__private::codegen_v2::inventory::submit! {
    RegistrationFragment::new(
        FragmentKind::Type,
        StaticFragmentIdentity::new(
            "builtin-registry-freeze-fixture",
            "explicit_registration",
            1,
            1,
            "type",
            1,
        ),
        explicit_runtime_identity,
        explicit_payload,
    )
}

/// Returns every non-generic built-in root that must be statically enumerated.
fn expected_builtin_roots() -> [&'static TypeDescriptor; 20] {
    [
        TypeDescriptor::of::<bool>(),
        TypeDescriptor::of::<char>(),
        TypeDescriptor::of::<i8>(),
        TypeDescriptor::of::<i16>(),
        TypeDescriptor::of::<i32>(),
        TypeDescriptor::of::<i64>(),
        TypeDescriptor::of::<i128>(),
        TypeDescriptor::of::<isize>(),
        TypeDescriptor::of::<u8>(),
        TypeDescriptor::of::<u16>(),
        TypeDescriptor::of::<u32>(),
        TypeDescriptor::of::<u64>(),
        TypeDescriptor::of::<u128>(),
        TypeDescriptor::of::<usize>(),
        TypeDescriptor::of::<f32>(),
        TypeDescriptor::of::<f64>(),
        TypeDescriptor::of::<String>(),
        TypeDescriptor::of::<str>(),
        TypeDescriptor::of::<()>(),
        TypeDescriptor::of::<dyn std::fmt::Debug>(),
    ]
}

/// Verifies registry initialization itself discovers every static built-in and
/// preserves exact root identity across all public indexes.
#[test]
fn test_builtin_registry_initializes_without_caller_prequery() {
    let registry = ReflectRegistry::initialize().expect("static built-in fragments must initialize");
    let expected = expected_builtin_roots();

    assert_eq!(registry.types().len(), expected.len() + 1);
    for descriptor in expected {
        assert!(std::ptr::eq(
            registry
                .get(descriptor.type_id())
                .expect("every required built-in must be indexed by TypeId"),
            descriptor,
        ));
        let type_name_matches: Vec<_> = registry.find_by_type_name(descriptor.type_name()).into_iter().collect();
        assert_eq!(type_name_matches.len(), 1);
        assert!(std::ptr::eq(type_name_matches[0], descriptor));
        let query_name_matches: Vec<_> = registry
            .find_by_query_name(descriptor.query_name())
            .into_iter()
            .collect();
        assert_eq!(query_name_matches.len(), 1);
        assert!(std::ptr::eq(query_name_matches[0], descriptor));
    }

    assert!(std::ptr::eq(
        registry
            .get(TypeId::of::<ExplicitRegistration>())
            .expect("explicit concrete registration must remain indexed"),
        TypeDescriptor::of::<ExplicitRegistration>(),
    ));
    let explicit = TypeDescriptor::of::<ExplicitRegistration>();
    assert!(
        registry
            .find_by_type_name(explicit.type_name())
            .into_iter()
            .any(|candidate| std::ptr::eq(candidate, explicit))
    );
    assert!(
        registry
            .find_by_query_name(explicit.query_name())
            .into_iter()
            .any(|candidate| std::ptr::eq(candidate, explicit))
    );
}

/// Verifies on-demand interning of generic composites cannot mutate any
/// public registry index after the frozen snapshot is published.
#[test]
fn test_builtin_registry_remains_frozen_after_composite_queries() {
    type Composite = Option<Vec<ExplicitRegistration>>;

    let registry = ReflectRegistry::initialize().expect("static built-in fragments must initialize");
    let public_surface = |registry: &'static ReflectRegistry| {
        registry
            .types()
            .iter()
            .map(|descriptor| {
                let address = *descriptor as *const TypeDescriptor as usize;
                let by_id = registry
                    .get(descriptor.type_id())
                    .map(|candidate| candidate as *const TypeDescriptor as usize);
                let by_type_name: Vec<_> = registry
                    .find_by_type_name(descriptor.type_name())
                    .into_iter()
                    .map(|candidate| candidate as *const TypeDescriptor as usize)
                    .collect();
                let by_query_name: Vec<_> = registry
                    .find_by_query_name(descriptor.query_name())
                    .into_iter()
                    .map(|candidate| candidate as *const TypeDescriptor as usize)
                    .collect();
                (descriptor.type_id(), address, by_id, by_type_name, by_query_name)
            })
            .collect::<Vec<_>>()
    };
    let before = public_surface(registry);
    let composite_type_name = std::any::type_name::<Composite>();

    assert!(registry.get(TypeId::of::<Composite>()).is_none());
    assert!(registry.find_by_type_name(composite_type_name).is_empty());
    assert!(registry.find_by_query_name(composite_type_name).is_empty());

    let composite = TypeDescriptor::of::<Composite>();
    assert_eq!(composite.type_id(), TypeId::of::<Composite>());

    let after = public_surface(registry);
    assert_eq!(after, before);
    assert!(registry.get(TypeId::of::<Composite>()).is_none());
    assert!(registry.find_by_type_name(composite_type_name).is_empty());
    assert!(registry.find_by_query_name(composite_type_name).is_empty());
}
