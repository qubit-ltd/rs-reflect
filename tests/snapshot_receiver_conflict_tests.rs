// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

//! Snapshot-only invocation preserves conflicts and never borrows global
//! adapters.
#![cfg(feature = "derive")]

use std::pin::Pin;
use std::rc::Rc;

use qubit_reflect::Invocation;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectRegistry;
use qubit_reflect::ReflectedOwned;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::capability::CapabilityDescriptor;
use qubit_reflect::capability::CapabilityKey;
use qubit_reflect::descriptor::MethodLookup;
use qubit_reflect::identity::CapabilityId;
use qubit_reflect::invoke::InvocationErrorKind;
use qubit_reflect::invoke::InvocationReceiver;
use qubit_reflect::invoke::receiver_adapter_key;
use qubit_reflect::reflect_impl;
use qubit_reflect::register_type_capabilities;
use qubit_reflect::registry::RegistrySnapshotBuilder;
use qubit_reflect::value::Local;

/// Declares the first conflicting intrinsic fact.
fn first<T: 'static>() -> CapabilityDescriptor {
    CapabilityDescriptor::with_adapter(
        CapabilityKey::new(CapabilityId::new("snapshot.conflict").unwrap()),
        std::any::TypeId::of::<T>(),
    )
}

/// Declares the same intrinsic ID through a separate provider.
fn second<T: 'static>() -> CapabilityDescriptor {
    first::<T>()
}

#[derive(Reflect)]
#[reflect(capabilities(first, second))]
struct Conflict<T> {
    value: T,
}

#[reflect_impl(specialize(T = u8))]
impl<T> Conflict<T> {
    /// Cannot execute while the receiver's intrinsic capabilities conflict.
    fn read(self: Pin<Rc<Self>>) -> u8 {
        99
    }
}

#[derive(Reflect)]
struct Global;

#[reflect_impl]
impl Global {
    /// Requires the receiver adapter registered only in the global registry.
    fn read(self: Pin<Rc<Self>>) -> u8 {
        7
    }
}

/// Accepts the exact globally registered receiver container.
fn global_adapter<'call>(
    receiver: InvocationReceiver<'call, Local>,
) -> Result<Pin<Rc<Global>>, InvocationReceiver<'call, Local>> {
    match receiver {
        InvocationReceiver::Owned(value) => value.downcast::<Pin<Rc<Global>>>().map_err(InvocationReceiver::Owned),
        receiver => Err(receiver),
    }
}

register_type_capabilities!(Global: [
    receiver_adapter_key::<Pin<Rc<Global>>, Local>() => global_adapter
]);

/// Copies only generated impl facts, without type membership or capabilities.
fn isolated(target: &'static TypeDescriptor) -> ReflectRegistry {
    let global = ReflectRegistry::initialize().unwrap();
    let mut builder = RegistrySnapshotBuilder::new();
    for implementation in global.implementations(target.type_id()) {
        builder.add_impl(implementation, implementation.definition().fragment_identity().clone());
    }
    builder.build().unwrap()
}

#[test]
fn test_intrinsic_conflict_survives_generated_snapshot_invocation() {
    let target = TypeDescriptor::of::<Conflict<u8>>();
    let registry = isolated(target);
    let expected = registry.capabilities(target).unwrap_err();
    let MethodLookup::Unique(method) = target.methods_named_in(&registry, "read") else {
        panic!("method")
    };
    let input = Invocation::owned(ReflectedOwned::new(Pin::new(Rc::new(Conflict { value: 1_u8 }))), []);
    let failure = method.invoke_local(&registry, input).unwrap().err().unwrap();
    let InvocationErrorKind::CapabilityResolution(actual) = failure.error().kind() else {
        panic!("original conflict")
    };
    assert_eq!(actual, &expected);
    let (receiver, arguments) = failure.into_recovery().into_parts();
    assert!(arguments.is_empty());
    let Some(InvocationReceiver::Owned(value)) = receiver else {
        panic!("owned input")
    };
    assert_eq!(
        value
            .downcast::<Pin<Rc<Conflict<u8>>>>()
            .unwrap_or_else(|_| panic!("receiver"))
            .value,
        1
    );
}

#[test]
fn test_missing_local_capability_does_not_use_a_valid_global_adapter() {
    let target = TypeDescriptor::of::<Global>();
    let global = ReflectRegistry::initialize().unwrap();
    assert!(
        global
            .capability(target, receiver_adapter_key::<Pin<Rc<Global>>, Local>())
            .unwrap()
            .is_some()
    );
    let registry = isolated(target);
    let MethodLookup::Unique(method) = target.methods_named_in(&registry, "read") else {
        panic!("method")
    };
    let input = Invocation::owned(ReflectedOwned::new(Pin::new(Rc::new(Global))), []);
    let failure = method.invoke_local(&registry, input).unwrap().err().unwrap();
    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::ReceiverAdapterUnavailable { .. }
    ));
    assert!(matches!(
        failure.recovery().receiver(),
        Some(InvocationReceiver::Owned(_))
    ));
}
