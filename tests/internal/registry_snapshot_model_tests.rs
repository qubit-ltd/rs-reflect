// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Shared public-API invariants for deterministic seeds and snapshot fuzzing.
use qubit_reflect::RegistryError;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::capability::CapabilityDescriptor;
use qubit_reflect::capability::CapabilityKey;
use qubit_reflect::identity::CapabilityId;
use qubit_reflect::identity::FragmentIdentity;
use qubit_reflect::registry::ReflectRegistry;
use qubit_reflect::registry::RegistrySnapshotBuilder;

const MAX_INPUT_BYTES: usize = 4_096;
const MAX_FRAGMENTS: usize = 32;
const MAX_SOURCES: u8 = 16;
const IDS: [&str; 8] = [
    "fuzz.a", "fuzz.b", "fuzz.c", "fuzz.d", "fuzz.e", "fuzz.f", "fuzz.g", "fuzz.h",
];

/// Selects from a fixed universe without per-input interning or leaked strings.
fn descriptor(index: u8) -> &'static TypeDescriptor {
    match index % 4 {
        0 => TypeDescriptor::of::<u8>(),
        1 => TypeDescriptor::of::<u16>(),
        2 => TypeDescriptor::of::<u32>(),
        _ => TypeDescriptor::of::<u64>(),
    }
}

/// Creates one of eight static adapter keys.
fn key(index: u8) -> CapabilityKey<u32> {
    CapabilityKey::new(CapabilityId::new(IDS[usize::from(index) % IDS.len()]).unwrap())
}

/// Creates one of sixteen fixed source identities.
fn source(index: u8) -> FragmentIdentity {
    let index = index % MAX_SOURCES;
    FragmentIdentity::new(
        "snapshot-fuzz",
        "fixed-source",
        u32::from(index),
        1,
        "fact",
        u64::from(index),
    )
}

/// Reconstructs facts in either order without duplicating validator logic.
fn build<'a>(operations: impl Iterator<Item = &'a [u8]>) -> Result<ReflectRegistry, RegistryError> {
    let mut builder = RegistrySnapshotBuilder::new();
    for operation in operations {
        let target = descriptor(operation[1]);
        if operation[0] % 2 == 0 {
            builder.add_type(target, source(operation[2]));
        } else {
            builder.add_type_capabilities(
                target,
                vec![CapabilityDescriptor::with_adapter(
                    key(operation[3]),
                    u32::from(operation[4]),
                )],
                source(operation[2]),
            );
        }
    }
    builder.build()
}

/// Captures observable membership and adapter values in a fixed order.
fn observations(registry: &ReflectRegistry) -> Vec<(bool, Vec<Option<u32>>)> {
    (0..4)
        .map(|index| {
            let target = descriptor(index);
            (
                registry.get(target.type_id()).is_some(),
                (0..8)
                    .map(|id| registry.capability(target, key(id)).unwrap().copied())
                    .collect(),
            )
        })
        .collect()
}

/// Checks order invariance, conflict atomicity, membership, and independence.
pub fn check(unbounded: &[u8]) {
    let data = &unbounded[..unbounded.len().min(MAX_INPUT_BYTES)];
    let operations: Vec<_> = data.chunks_exact(5).take(MAX_FRAGMENTS).collect();
    let first = build(operations.iter().copied());
    let reversed = build(operations.iter().rev().copied());
    match (first, reversed) {
        (Ok(first), Ok(reversed)) => {
            assert_eq!(observations(&first), observations(&reversed))
        }
        (Err(first), Err(reversed)) => {
            assert!(!first.to_string().is_empty());
            assert!(!reversed.to_string().is_empty());
        }
        _ => panic!("validity must not depend on insertion order"),
    }
    let value = u32::from(data.first().copied().unwrap_or(0));
    let isolated = |value| {
        let mut builder = RegistrySnapshotBuilder::new();
        builder.add_type_capabilities(
            descriptor(0),
            vec![CapabilityDescriptor::with_adapter(key(0), value)],
            source(0),
        );
        builder.build().unwrap()
    };
    let first = isolated(value);
    let second = isolated(value + 1);
    assert!(first.types().is_empty());
    assert!(second.types().is_empty());
    let before = observations(&first);
    let mut duplicate = RegistrySnapshotBuilder::new();
    duplicate
        .add_type(descriptor(0), source(0))
        .add_type(descriptor(0), source(0));
    assert!(duplicate.build().is_err());
    let mut conflicting = RegistrySnapshotBuilder::new();
    for id in 0..2 {
        conflicting.add_type_capabilities(
            descriptor(0),
            vec![CapabilityDescriptor::with_adapter(key(0), u32::from(id))],
            source(id),
        );
    }
    assert!(conflicting.build().is_err());
    assert_eq!(observations(&first), before);
    assert_eq!(first.capability(descriptor(0), key(0)).unwrap(), Some(&value));
    assert_eq!(second.capability(descriptor(0), key(0)).unwrap(), Some(&(value + 1)));
}
