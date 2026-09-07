// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Invalid intrinsic facts on unregistered monomorphs must remain errors.
#![cfg(feature = "derive")]
use qubit_reflect::Reflect;
use qubit_reflect::ReflectRegistry;
use qubit_reflect::ReflectedOwned;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::capability::CapabilityConflictKind;
use qubit_reflect::capability::CapabilityDescriptor;
use qubit_reflect::capability::CapabilityKey;
use qubit_reflect::capability::CapabilityLookup;
use qubit_reflect::identity::CapabilityId;
use qubit_reflect::registry::RegistrySnapshotBuilder;

/// The shared capability contract intentionally registered twice.
fn key() -> CapabilityKey<fn()> {
    CapabilityKey::new(CapabilityId::new("example.generic_conflict").unwrap())
}
/// A harmless executable adapter.
fn adapter() {}
/// A fact-only contract used to exercise strict capability lookup.
fn fact_key() -> CapabilityKey<fn()> {
    CapabilityKey::new(CapabilityId::new("example.generic_fact").unwrap())
}
/// Declares the fact-only contract for each concrete monomorph.
#[allow(
    clippy::extra_unused_type_parameters,
    reason = "derive capability providers receive the concrete type parameter"
)]
fn fact<T: 'static>() -> CapabilityDescriptor {
    CapabilityDescriptor::without_adapter(fact_key())
}
/// First declaration of the conflicting ID.
fn first<T: 'static>() -> CapabilityDescriptor {
    *FAIL_COUNTS
        .lock()
        .unwrap()
        .entry(std::any::TypeId::of::<T>())
        .or_default() += 1;
    CapabilityDescriptor::with_adapter(key(), adapter as fn())
}
/// Second declaration of the conflicting ID.
#[allow(
    clippy::extra_unused_type_parameters,
    reason = "derive capability providers receive the concrete type parameter"
)]
fn second<T: 'static>() -> CapabilityDescriptor {
    CapabilityDescriptor::with_adapter(key(), adapter as fn())
}
#[derive(Reflect)]
#[reflect(capabilities(first, second))]
struct Conflict<T> {
    value: T,
}

#[test]
fn test_unregistered_generic_conflict_is_not_absence() {
    let registry = ReflectRegistry::initialize().unwrap();
    let descriptor = TypeDescriptor::of::<Conflict<u32>>();
    assert!(registry.get(descriptor.type_id()).is_none());
    let error = registry.capability(descriptor, key()).unwrap_err();
    assert_eq!(error.kind(), CapabilityConflictKind::DuplicateId);
    assert_eq!(error.id().as_str(), "example.generic_conflict");
    assert_eq!(registry.capabilities(descriptor).unwrap_err(), error);
    assert_eq!(
        registry
            .capability_by_id(descriptor, "example.generic_conflict")
            .unwrap_err(),
        error
    );
}

/// Deliberately assigns the same ID to a different adapter contract.
#[allow(
    clippy::extra_unused_type_parameters,
    reason = "derive capability providers receive the concrete type parameter"
)]
fn different_contract<T: 'static>() -> CapabilityDescriptor {
    let key = CapabilityKey::new(CapabilityId::new("example.generic_conflict").unwrap());
    CapabilityDescriptor::with_adapter(key, 17_usize)
}

#[derive(Reflect)]
#[reflect(capabilities(first, different_contract))]
struct Mismatch<T> {
    value: T,
}

#[test]
fn test_mismatch_preserves_both_contracts_and_snapshot_membership() {
    use std::any::TypeId;
    let registry = ReflectRegistry::initialize().unwrap();
    let before = registry.types().len();
    let descriptor = TypeDescriptor::of::<Mismatch<String>>();
    let error = registry.capabilities(descriptor).unwrap_err();
    assert_eq!(error.kind(), CapabilityConflictKind::AdapterTypeMismatch);
    let actual = [error.first_adapter_type(), error.second_adapter_type()];
    assert!(actual.contains(&TypeId::of::<fn()>()));
    assert!(actual.contains(&TypeId::of::<usize>()));
    assert_eq!(registry.types().len(), before);
    assert!(registry.type_source(descriptor.type_id()).is_none());
    assert_eq!(registry.capabilities(descriptor).unwrap_err(), error);
}

/// Counts a successful intrinsic factory independently for every monomorph.
fn counted<T: 'static>() -> CapabilityDescriptor {
    let mut counts = COUNTS.lock().unwrap();
    *counts.entry(std::any::TypeId::of::<T>()).or_default() += 1;
    CapabilityDescriptor::with_adapter(key(), adapter as fn())
}
static FAIL_COUNTS: std::sync::Mutex<std::collections::BTreeMap<std::any::TypeId, usize>> =
    std::sync::Mutex::new(std::collections::BTreeMap::new());
static COUNTS: std::sync::Mutex<std::collections::BTreeMap<std::any::TypeId, usize>> =
    std::sync::Mutex::new(std::collections::BTreeMap::new());

#[derive(Reflect)]
#[reflect(capabilities(counted, fact))]
struct Counted<T> {
    value: T,
}

#[test]
fn test_concurrent_monomorph_initialization_is_cached_and_missing_keys_remain_absent() {
    let registry = ReflectRegistry::initialize().unwrap();
    let before = registry.types().len();
    std::thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| {
                for _ in 0..20 {
                    let descriptor = TypeDescriptor::of::<Counted<u32>>();
                    assert!(registry.capability(descriptor, key()).unwrap().is_some());
                    assert!(registry.capabilities(TypeDescriptor::of::<Conflict<u64>>()).is_err());
                }
            });
        }
    });
    assert_eq!(
        FAIL_COUNTS
            .lock()
            .unwrap()
            .get(&std::any::TypeId::of::<Conflict<u64>>()),
        Some(&1)
    );
    let other = TypeDescriptor::of::<Counted<String>>();
    assert!(registry.capability(other, key()).unwrap().is_some());
    let counts = COUNTS.lock().unwrap();
    assert_eq!(counts.get(&std::any::TypeId::of::<Counted<u32>>()), Some(&1));
    assert_eq!(counts.get(&std::any::TypeId::of::<Counted<String>>()), Some(&1));
    let missing: CapabilityKey<fn()> = CapabilityKey::new(CapabilityId::new("example.absent").unwrap());
    assert!(registry.capability(other, missing).unwrap().is_none());
    assert!(registry.capability_by_id(other, "invalid!").unwrap().is_none());
    let wrong: CapabilityKey<usize> = CapabilityKey::new(*key().id());
    assert!(registry.capability(other, wrong).unwrap().is_none());
    assert!(matches!(
        registry.capability_lookup(other, missing).unwrap(),
        CapabilityLookup::Missing
    ));
    assert!(matches!(
        registry.capability_lookup(other, fact_key()).unwrap(),
        CapabilityLookup::FactOnly(_)
    ));
    assert!(matches!(
        registry.capability_lookup(other, wrong).unwrap(),
        CapabilityLookup::AdapterTypeMismatch { .. }
    ));
    assert!(matches!(
        registry.capability_lookup(other, key()).unwrap(),
        CapabilityLookup::Found(_)
    ));
    assert_eq!(registry.types().len(), before);
}

#[test]
fn test_receiver_capability_conflict_preserves_validated_inputs() {
    let registry = RegistrySnapshotBuilder::new().build().unwrap();
    use qubit_reflect::identity::FragmentIdentity;
    use qubit_reflect::identity::MemberId;
    use qubit_reflect::invoke::ArgumentExpectation;
    use qubit_reflect::invoke::Invocation;
    use qubit_reflect::invoke::InvocationArg;
    use qubit_reflect::invoke::InvocationErrorKind;
    use qubit_reflect::invoke::ReceiverExpectation;
    let identity = MemberId::new(
        "example::Receiver",
        "method",
        1,
        FragmentIdentity::new("example", "example::Receiver", 1, 1, "method", 1),
    );
    let absent = ReceiverExpectation::none();
    let unit = ReceiverExpectation::owned::<()>();
    assert_eq!(absent.type_id(), None);
    assert_eq!(absent.type_name(), None);
    assert_eq!(unit.type_id(), Some(std::any::TypeId::of::<()>()));
    assert_eq!(unit.type_name(), Some(std::any::type_name::<()>()));
    let invocation = Invocation::associated([InvocationArg::Owned(ReflectedOwned::new(7_u8))]);
    let validated = invocation
        .validate(
            &identity,
            ReceiverExpectation::none(),
            &[ArgumentExpectation::owned::<u8>()],
        )
        .unwrap();
    let failure = match validated.adapt_receiver_in::<()>(&registry, &identity, TypeDescriptor::of::<Conflict<u16>>()) {
        Err(failure) => failure,
        Ok(_) => panic!("invalid capabilities must reject adaptation"),
    };
    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::CapabilityResolution(_)
    ));
    let (_, arguments) = failure.into_recovery().into_parts();
    let InvocationArg::Owned(value) = arguments.into_vec().pop().unwrap() else {
        panic!("owned argument")
    };
    assert_eq!(
        value.downcast::<u8>().unwrap_or_else(|_| panic!("exact argument type")),
        7
    );
}
