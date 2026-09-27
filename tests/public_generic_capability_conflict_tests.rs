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
    CapabilityKey::new(CapabilityId::new("example.generic_conflict").expect("fixture ID is valid"))
}
/// A harmless executable adapter.
fn adapter() {}
/// A fact-only contract used to exercise strict capability lookup.
fn fact_key() -> CapabilityKey<fn()> {
    CapabilityKey::new(CapabilityId::new("example.generic_fact").expect("fixture ID is valid"))
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
        .expect("the failure counter mutex must not be poisoned")
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
    let registry = ReflectRegistry::initialize().expect("fixture registrations must initialize");
    let descriptor = TypeDescriptor::of::<Conflict<u32>>();
    assert!(registry.get(descriptor.type_id()).is_none());
    let error = registry
        .capability_lookup(descriptor, key())
        .expect_err("duplicate intrinsic capability IDs must be reported");
    assert_eq!(error.kind(), CapabilityConflictKind::DuplicateId);
    assert_eq!(error.id().as_str(), "example.generic_conflict");
    assert_eq!(
        registry
            .capabilities(descriptor)
            .expect_err("the monomorph's capability conflict must be reported"),
        error
    );
    assert_eq!(
        registry
            .capability_by_id(descriptor, "example.generic_conflict")
            .expect_err("the same capability conflict must survive ID lookup"),
        error
    );
}

/// Deliberately assigns the same ID to a different adapter contract.
#[allow(
    clippy::extra_unused_type_parameters,
    reason = "derive capability providers receive the concrete type parameter"
)]
fn different_contract<T: 'static>() -> CapabilityDescriptor {
    let key = CapabilityKey::new(CapabilityId::new("example.generic_conflict").expect("fixture ID is valid"));
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
    let registry = ReflectRegistry::initialize().expect("fixture registrations must initialize");
    let before = registry.types().len();
    let descriptor = TypeDescriptor::of::<Mismatch<String>>();
    let error = registry
        .capabilities(descriptor)
        .expect_err("the adapter type mismatch must be reported");
    assert_eq!(error.kind(), CapabilityConflictKind::AdapterTypeMismatch);
    let actual = [error.first_adapter_type(), error.second_adapter_type()];
    assert!(actual.contains(&TypeId::of::<fn()>()));
    assert!(actual.contains(&TypeId::of::<usize>()));
    assert_eq!(registry.types().len(), before);
    assert!(registry.type_source(descriptor.type_id()).is_none());
    assert_eq!(
        registry
            .capabilities(descriptor)
            .expect_err("the same adapter type mismatch must be reported"),
        error
    );
}

/// Counts a successful intrinsic factory independently for every monomorph.
fn counted<T: 'static>() -> CapabilityDescriptor {
    let mut counts = COUNTS.lock().expect("the success counter mutex must not be poisoned");
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
    let registry = ReflectRegistry::initialize().expect("fixture registrations must initialize");
    let before = registry.types().len();
    std::thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| {
                for _ in 0..20 {
                    let descriptor = TypeDescriptor::of::<Counted<u32>>();
                    assert!(
                        registry
                            .capability(descriptor, key())
                            .expect("the successful monomorph capability must resolve")
                            .is_some()
                    );
                    assert!(registry.capabilities(TypeDescriptor::of::<Conflict<u64>>()).is_err());
                }
            });
        }
    });
    assert_eq!(
        FAIL_COUNTS
            .lock()
            .expect("the failure counter mutex must not be poisoned")
            .get(&std::any::TypeId::of::<Conflict<u64>>()),
        Some(&1)
    );
    let other = TypeDescriptor::of::<Counted<String>>();
    assert!(
        registry
            .capability(other, key())
            .expect("the other monomorph capability must resolve")
            .is_some()
    );
    let counts = COUNTS.lock().expect("the success counter mutex must not be poisoned");
    assert_eq!(counts.get(&std::any::TypeId::of::<Counted<u32>>()), Some(&1));
    assert_eq!(counts.get(&std::any::TypeId::of::<Counted<String>>()), Some(&1));
    let missing: CapabilityKey<fn()> =
        CapabilityKey::new(CapabilityId::new("example.absent").expect("fixture ID is valid"));
    assert!(
        registry
            .capability(other, missing)
            .expect("missing typed capability lookup is valid")
            .is_none()
    );
    assert!(
        registry
            .capability_by_id(other, "invalid!")
            .expect("invalid external IDs are represented as absent")
            .is_none()
    );
    let wrong: CapabilityKey<usize> = CapabilityKey::new(*key().id());
    assert!(registry.capability(other, wrong).is_err());
    assert!(matches!(
        registry
            .capability_lookup(other, missing)
            .expect("missing capability lookup is valid"),
        CapabilityLookup::Missing
    ));
    assert!(matches!(
        registry
            .capability_lookup(other, fact_key())
            .expect("fact-only lookup is valid"),
        CapabilityLookup::FactOnly(_)
    ));
    assert!(matches!(
        registry
            .capability_lookup(other, wrong)
            .expect("adapter mismatch is a lookup result"),
        CapabilityLookup::AdapterTypeMismatch { .. }
    ));
    assert!(matches!(
        registry
            .capability_lookup(other, key())
            .expect("found capability lookup is valid"),
        CapabilityLookup::Found(_)
    ));
    assert_eq!(registry.types().len(), before);
}

#[test]
fn test_receiver_capability_conflict_preserves_validated_inputs() {
    use qubit_reflect::identity::FragmentIdentity;
    use qubit_reflect::identity::MemberId;
    use qubit_reflect::invoke::ArgumentExpectation;
    use qubit_reflect::invoke::Invocation;
    use qubit_reflect::invoke::InvocationArg;
    use qubit_reflect::invoke::InvocationErrorKind;
    use qubit_reflect::invoke::ReceiverExpectation;

    let registry = RegistrySnapshotBuilder::new()
        .build()
        .expect("an empty isolated snapshot must build");
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
        .expect("the exact owned argument must validate");
    let failure = match validated.adapt_receiver_in::<()>(&registry, &identity, TypeDescriptor::of::<Conflict<u16>>()) {
        Err(failure) => failure,
        Ok(_) => panic!("invalid capabilities must reject adaptation"),
    };
    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::CapabilityResolution(_)
    ));
    let (_, arguments) = failure.into_recovery().into_parts();
    let InvocationArg::Owned(value) = arguments
        .into_vec()
        .pop()
        .expect("the validated invocation must retain its argument")
    else {
        panic!("owned argument")
    };
    assert_eq!(
        value.downcast::<u8>().unwrap_or_else(|_| panic!("exact argument type")),
        7
    );
}
