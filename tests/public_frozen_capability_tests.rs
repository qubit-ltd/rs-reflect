// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Frozen lookups must never re-enter intrinsic capability factories.

use std::sync::OnceLock;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use qubit_reflect::__private::codegen_v2::descriptor::opaque_root;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectRegistry;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::capability::CapabilityConflict;
use qubit_reflect::capability::CapabilityDescriptor;
use qubit_reflect::capability::CapabilityKey;
use qubit_reflect::capability::TypeCapabilities;
use qubit_reflect::identity::CapabilityId;
use qubit_reflect::register_reflected_type;

struct Empty;
struct Populated;
static EMPTY_CALLS: AtomicUsize = AtomicUsize::new(0);
static POPULATED_CALLS: AtomicUsize = AtomicUsize::new(0);

fn key() -> CapabilityKey<usize> {
    CapabilityKey::new(CapabilityId::new("example.frozen").unwrap())
}

fn empty() -> Result<&'static TypeCapabilities, CapabilityConflict> {
    EMPTY_CALLS.fetch_add(1, Ordering::SeqCst);
    static SET: OnceLock<TypeCapabilities> = OnceLock::new();
    Ok(SET.get_or_init(|| TypeCapabilities::try_new(vec![]).unwrap()))
}

fn populated() -> Result<&'static TypeCapabilities, CapabilityConflict> {
    POPULATED_CALLS.fetch_add(1, Ordering::SeqCst);
    static SET: OnceLock<TypeCapabilities> = OnceLock::new();
    Ok(SET.get_or_init(|| TypeCapabilities::try_new(vec![CapabilityDescriptor::with_adapter(key(), 3)]).unwrap()))
}

impl Reflect for Empty {
    fn type_descriptor() -> &'static TypeDescriptor {
        static DESCRIPTOR: TypeDescriptor = opaque_root::<Empty>("Empty").with_capabilities(empty);
        &DESCRIPTOR
    }
}

impl Reflect for Populated {
    fn type_descriptor() -> &'static TypeDescriptor {
        static DESCRIPTOR: TypeDescriptor = opaque_root::<Populated>("Populated").with_capabilities(populated);
        &DESCRIPTOR
    }
}

register_reflected_type!(Empty);
register_reflected_type!(Populated);

#[test]
fn test_frozen_empty_and_populated_queries_never_execute_factories() {
    let registry = ReflectRegistry::initialize().unwrap();
    let before = (
        EMPTY_CALLS.load(Ordering::SeqCst),
        POPULATED_CALLS.load(Ordering::SeqCst),
    );
    assert!(before.0 > 0 && before.1 > 0);
    for _ in 0..10 {
        for descriptor in [Empty::type_descriptor(), Populated::type_descriptor()] {
            let _ = registry.capabilities(descriptor).unwrap();
            let _ = registry.capability(descriptor, key()).unwrap();
            let _ = registry.capability_by_id(descriptor, "example.frozen").unwrap();
        }
        assert_eq!(registry.types_with_capability(key()).count(), 1);
    }
    assert_eq!(
        before,
        (
            EMPTY_CALLS.load(Ordering::SeqCst),
            POPULATED_CALLS.load(Ordering::SeqCst)
        )
    );
}
