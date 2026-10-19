// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0 (the "License");
//    you may not use this file except in compliance with the License.
// =============================================================================

//! Integration tests for built-in vNext type shapes.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::hash::{BuildHasherDefault, DefaultHasher};
use std::sync::Arc;

use qubit_reflect::TypeDescriptor;
use qubit_reflect::descriptor::TypeKind;

/// Verifies built-in scalar and text types retain their structural families.
#[test]
fn test_builtin_scalar_and_text_type_kinds() {
    assert!(matches!(
        TypeDescriptor::of::<bool>().kind(),
        TypeKind::Primitive(_)
    ));
    assert!(matches!(
        TypeDescriptor::of::<char>().kind(),
        TypeKind::Primitive(_)
    ));
    assert!(matches!(
        TypeDescriptor::of::<u64>().kind(),
        TypeKind::Primitive(_)
    ));
    assert!(matches!(
        TypeDescriptor::of::<String>().kind(),
        TypeKind::Text(_)
    ));
    assert!(matches!(
        TypeDescriptor::of::<str>().kind(),
        TypeKind::Text(_)
    ));
}

/// Verifies built-in collections preserve their source-level shape families.
#[test]
fn test_builtin_container_type_kinds() {
    assert!(matches!(
        TypeDescriptor::of::<Option<u32>>().kind(),
        TypeKind::Optional(_)
    ));
    assert!(matches!(
        TypeDescriptor::of::<Vec<String>>().kind(),
        TypeKind::Sequence(_)
    ));
    assert!(matches!(
        TypeDescriptor::of::<HashSet<u16>>().kind(),
        TypeKind::Set(_)
    ));
    assert!(matches!(
        TypeDescriptor::of::<BTreeSet<u16>>().kind(),
        TypeKind::Set(_)
    ));
    assert!(matches!(
        TypeDescriptor::of::<BTreeMap<u8, String>>().kind(),
        TypeKind::Map(_)
    ));

    type CustomHashMap = HashMap<u8, String, BuildHasherDefault<DefaultHasher>>;
    assert!(matches!(
        TypeDescriptor::of::<CustomHashMap>().kind(),
        TypeKind::Map(_)
    ));
}

/// Verifies built-in composite and pointer types retain their shape families.
#[test]
fn test_builtin_composite_and_pointer_type_kinds() {
    assert!(matches!(
        TypeDescriptor::of::<()>().kind(),
        TypeKind::Tuple(_)
    ));
    assert!(matches!(
        TypeDescriptor::of::<(u8, String)>().kind(),
        TypeKind::Tuple(_)
    ));
    assert!(matches!(
        TypeDescriptor::of::<[u8; 3]>().kind(),
        TypeKind::Array(_)
    ));
    assert!(matches!(
        TypeDescriptor::of::<Arc<u32>>().kind(),
        TypeKind::SmartPointer(_)
    ));
    assert!(matches!(
        TypeDescriptor::of::<&'static u32>().kind(),
        TypeKind::Reference(_)
    ));
    assert!(matches!(
        TypeDescriptor::of::<*const u32>().kind(),
        TypeKind::RawPointer(_)
    ));
    assert!(matches!(
        TypeDescriptor::of::<[u32]>().kind(),
        TypeKind::Slice(_)
    ));
    assert!(matches!(
        TypeDescriptor::of::<fn(u8) -> String>().kind(),
        TypeKind::Function(_)
    ));
    assert!(matches!(
        TypeDescriptor::of::<dyn std::fmt::Debug>().kind(),
        TypeKind::TraitObject(_)
    ));
}

/// Verifies generic specializations keep distinct Rust identities.
#[test]
fn test_builtin_specializations_keep_distinct_type_identities() {
    let numbers = TypeDescriptor::of::<Vec<u32>>();
    let strings = TypeDescriptor::of::<Vec<String>>();

    assert_ne!(numbers.type_id(), strings.type_id());
    assert_eq!(numbers.type_id(), std::any::TypeId::of::<Vec<u32>>());
    assert_eq!(strings.type_id(), std::any::TypeId::of::<Vec<String>>());
}

/// Verifies descriptors can be obtained concurrently for the same built-in
/// type without changing its exact Rust identity.
#[test]
fn test_builtin_descriptors_are_stable_under_concurrent_lookup() {
    let workers: Vec<_> = (0..16)
        .map(|_| std::thread::spawn(TypeDescriptor::of::<Vec<u32>>))
        .collect();

    for worker in workers {
        let descriptor = worker.join().expect("descriptor lookup must not panic");
        assert_eq!(descriptor.type_id(), std::any::TypeId::of::<Vec<u32>>());
    }
}
