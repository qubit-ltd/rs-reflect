// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Public capability projection integration tests.

use std::any::TypeId;

use qubit_reflect::__private::codegen_v3::descriptor::opaque_root;
use qubit_reflect::capability::CapabilityDescriptor;
use qubit_reflect::capability::CapabilityKey;
use qubit_reflect::capability::CapabilityLookup;
use qubit_reflect::capability::CapabilityOrigin;
use qubit_reflect::capability::clone_key;
use qubit_reflect::descriptor::Reflect;
use qubit_reflect::descriptor::TypeDescriptor;
use qubit_reflect::identity::CapabilityId;
use qubit_reflect::identity::FragmentIdentity;
use qubit_reflect::register_reflected_type;
use qubit_reflect::register_type_capabilities;
use qubit_reflect::registry::ReflectRegistry;
use qubit_reflect::registry::RegistrySnapshotBuilder;

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
    let capabilities = registry
        .capabilities(RegisteredCapability::type_descriptor())
        .expect("valid capability declarations");

    assert!(capabilities.contains(clone_key()));
    assert!(
        registry
            .capability_by_id(RegisteredCapability::type_descriptor(), "qubit.reflect.clone",)
            .expect("valid capability declarations")
            .is_some()
    );
    assert!(
        registry
            .capability_by_id(RegisteredCapability::type_descriptor(), "not-valid!")
            .expect("valid capability declarations")
            .is_none()
    );
    let matches: Vec<_> = registry
        .type_capability_members(clone_key())
        .map(|member| member.target())
        .collect();
    assert!(
        matches
            .iter()
            .any(|descriptor| std::ptr::eq(*descriptor, RegisteredCapability::type_descriptor()))
    );
    assert!(registry.type_source(TypeId::of::<RegisteredCapability>()).is_some());
    assert_eq!(registry.types_with_identity().count(), registry.types().len());
}

/// Verifies member iteration retains every typed lookup state and its source.
#[test]
fn test_snapshot_type_capability_members_retain_lookup_and_provenance() {
    let found_key = CapabilityKey::new(CapabilityId::new("example.member.found").expect("valid ID"));
    let fact_key = CapabilityKey::<String>::new(CapabilityId::new("example.member.fact").expect("valid ID"));
    let mismatch_key = CapabilityKey::<usize>::new(CapabilityId::new("example.member.mismatch").expect("valid ID"));
    let missing_key = CapabilityKey::<usize>::new(CapabilityId::new("example.member.missing").expect("valid ID"));
    let first = TypeDescriptor::of::<u16>();
    let second = TypeDescriptor::of::<u32>();
    let capability_source = FragmentIdentity::new("fixture", "members", 30, 1, "capability", 30);
    let mut builder = RegistrySnapshotBuilder::new();
    builder.add_type(first, FragmentIdentity::new("fixture", "members", 10, 1, "type", 10));
    builder.add_type_capabilities(
        first,
        vec![
            CapabilityDescriptor::with_adapter(found_key, 42_usize),
            CapabilityDescriptor::without_adapter(fact_key),
            CapabilityDescriptor::without_adapter(CapabilityKey::<u64>::new(*mismatch_key.id())),
        ],
        capability_source.clone(),
    );
    builder.add_type(second, FragmentIdentity::new("fixture", "members", 20, 1, "type", 20));
    builder.add_type_capabilities(
        second,
        vec![CapabilityDescriptor::with_adapter(found_key, 43_usize)],
        FragmentIdentity::new("fixture", "members", 31, 1, "capability", 31),
    );
    let capability_only = TypeDescriptor::of::<u64>();
    builder.add_type_capabilities(
        capability_only,
        vec![CapabilityDescriptor::with_adapter(found_key, 44_usize)],
        FragmentIdentity::new("fixture", "members", 32, 1, "capability", 32),
    );
    let registry = builder.build().expect("valid capability member snapshot");

    let found_members: Vec<_> = registry.type_capability_members(found_key).collect();
    assert_eq!(found_members.len(), 2);
    assert!(std::ptr::eq(found_members[0].target(), first));
    assert!(std::ptr::eq(found_members[1].target(), second));
    assert!(matches!(
        found_members[0].lookup(),
        CapabilityLookup::Found(value) if **value == 42,
    ));
    assert_eq!(found_members[0].source(), &capability_source);
    assert_eq!(
        found_members[0].origin(),
        &CapabilityOrigin::Registered {
            source: capability_source,
        },
    );
    let fact_member = registry
        .type_capability_members(fact_key)
        .next()
        .expect("fact-only capability member");
    assert!(matches!(fact_member.lookup(), CapabilityLookup::FactOnly(_)));
    let mismatch_member = registry
        .type_capability_members(mismatch_key)
        .next()
        .expect("mismatched capability member");
    assert!(matches!(
        mismatch_member.lookup(),
        CapabilityLookup::AdapterTypeMismatch { .. }
    ));
    assert!(registry.type_capability_members(missing_key).next().is_none());
    assert_eq!(registry.capability_only_type_targets(found_key.id().as_str()).len(), 1);
}
