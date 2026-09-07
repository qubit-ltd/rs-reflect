// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Generated receiver adapters resolve capabilities in the explicit snapshot.
#![cfg(feature = "derive")]

use std::pin::Pin;
use std::rc::Rc;

use qubit_reflect::Invocation;
use qubit_reflect::InvocationOutput;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectedOwned;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::capability::CapabilityDescriptor;
use qubit_reflect::capability::CapabilityKey;
use qubit_reflect::descriptor::MethodLookup;
use qubit_reflect::identity::FragmentIdentity;
use qubit_reflect::invoke::InvocationArg;
use qubit_reflect::invoke::InvocationBinding;
use qubit_reflect::invoke::InvocationErrorKind;
use qubit_reflect::invoke::InvocationReceiver;
use qubit_reflect::invoke::ReceiverAdapter;
use qubit_reflect::invoke::receiver_adapter_key;
use qubit_reflect::reflect_impl;
use qubit_reflect::registry::ReflectRegistry;
use qubit_reflect::registry::RegistrySnapshotBuilder;
use qubit_reflect::value::Local;

#[derive(Reflect)]
struct Counter {
    value: u32,
}

#[reflect_impl]
impl Counter {
    /// Returns a value selected by the snapshot's receiver adapter.
    fn read(self: Pin<Rc<Self>>) -> u32 {
        self.value
    }

    /// Keeps a non-Send receiver alive inside the local future.
    async fn read_async(self: Pin<Rc<Self>>) -> u32 {
        std::future::ready(()).await;
        self.value
    }

    /// Carries named and positional inputs across receiver adaptation.
    fn read_inputs(self: Pin<Rc<Self>>, first: u8, second: u16) -> u32 {
        self.value + u32::from(first) + u32::from(second)
    }
}

/// Replaces the accepted receiver with a fixed value for snapshot A.
fn first<'call>(
    receiver: InvocationReceiver<'call, Local>,
) -> Result<Pin<Rc<Counter>>, InvocationReceiver<'call, Local>> {
    match receiver {
        InvocationReceiver::Owned(value) => value
            .downcast::<Pin<Rc<Counter>>>()
            .map(|_| Pin::new(Rc::new(Counter { value: 11 })))
            .map_err(InvocationReceiver::Owned),
        receiver => Err(receiver),
    }
}

/// Replaces the accepted receiver with a distinct value for snapshot B.
fn second<'call>(
    receiver: InvocationReceiver<'call, Local>,
) -> Result<Pin<Rc<Counter>>, InvocationReceiver<'call, Local>> {
    first(receiver).map(|_| Pin::new(Rc::new(Counter { value: 22 })))
}

/// Rejects without consuming any part of the caller's receiver.
fn reject<'call>(
    receiver: InvocationReceiver<'call, Local>,
) -> Result<Pin<Rc<Counter>>, InvocationReceiver<'call, Local>> {
    Err(receiver)
}

/// Builds an impl-only snapshot with optional receiver capabilities.
fn snapshot(adapter: Option<ReceiverAdapter<Pin<Rc<Counter>>, Local>>) -> ReflectRegistry {
    let global = ReflectRegistry::initialize().expect("valid global declarations");
    let descriptor = TypeDescriptor::of::<Counter>();
    let mut builder = RegistrySnapshotBuilder::new();
    for implementation in global.implementations(descriptor.type_id()) {
        builder.add_impl(implementation, implementation.definition().fragment_identity().clone());
    }
    if let Some(adapter) = adapter {
        builder.add_type_capabilities(
            descriptor,
            vec![CapabilityDescriptor::with_adapter(
                receiver_adapter_key::<Pin<Rc<Counter>>, Local>(),
                adapter,
            )],
            FragmentIdentity::new("snapshot-call-test", "receiver", 1, 1, "capability", 1),
        );
    }
    builder.build().expect("valid isolated snapshot")
}

/// Supplies an owned pinned receiver without global capability registration.
fn invocation() -> Invocation<'static, Local> {
    Invocation::owned(ReflectedOwned::new(Pin::new(Rc::new(Counter { value: 1 }))), [])
}

/// Executes the generated method using the same snapshot used for lookup.
fn call(registry: &ReflectRegistry) -> u32 {
    let MethodLookup::Unique(method) = TypeDescriptor::of::<Counter>().methods_named_in(registry, "read") else {
        panic!("the snapshot must contain the generated method")
    };
    let output = method
        .invoke_local(registry, invocation())
        .expect("safe signature must generate an adapter without inventory capability probing")
        .expect("snapshot receiver capability must be selected");
    let InvocationOutput::Owned(value) = output else {
        panic!("owned u32 output")
    };
    value.downcast::<u32>().unwrap_or_else(|_| panic!("exact output type"))
}

#[test]
fn test_generated_receiver_uses_each_snapshot_without_global_registration() {
    let first = snapshot(Some(first));
    let second = snapshot(Some(second));
    assert!(
        first.types().is_empty(),
        "impl/capability-only must not add type membership"
    );
    assert_eq!(call(&first), 11);
    assert_eq!(call(&second), 22);
    assert_eq!(
        call(&first),
        11,
        "another snapshot must not overwrite receiver resolution"
    );
}

#[test]
fn test_missing_snapshot_capability_recovers_the_original_receiver() {
    let registry = snapshot(None);
    let MethodLookup::Unique(method) = TypeDescriptor::of::<Counter>().methods_named_in(&registry, "read") else {
        panic!("generated method exists")
    };
    let failure = method
        .invoke_local(&registry, invocation())
        .expect("static adapter exists")
        .err()
        .expect("missing receiver capability must fail dynamically");
    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::ReceiverAdapterUnavailable { .. }
    ));
    let (receiver, arguments) = failure.into_recovery().into_parts();
    assert!(arguments.is_empty());
    let Some(InvocationReceiver::Owned(value)) = receiver else {
        panic!("original owned receiver")
    };
    let value = value
        .downcast::<Pin<Rc<Counter>>>()
        .unwrap_or_else(|_| panic!("exact receiver type"));
    assert_eq!(value.value, 1);
}

#[test]
fn test_snapshot_receiver_rejection_preserves_input() {
    let registry = snapshot(Some(reject));
    let MethodLookup::Unique(method) = TypeDescriptor::of::<Counter>().methods_named_in(&registry, "read") else {
        panic!("generated method exists")
    };
    let failure = method
        .invoke_local(&registry, invocation())
        .expect("static adapter exists")
        .err()
        .expect("adapter rejects receiver");
    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::ReceiverAdapterRejected { .. }
    ));
    assert!(matches!(
        failure.recovery().receiver(),
        Some(InvocationReceiver::Owned(_))
    ));
}

#[test]
fn test_owned_output_and_local_future_outlive_the_selected_registry() {
    let output = {
        let registry = snapshot(Some(first));
        let MethodLookup::Unique(method) = TypeDescriptor::of::<Counter>().methods_named_in(&registry, "read") else {
            panic!("generated method")
        };
        method.invoke_local(&registry, invocation()).unwrap().unwrap()
    };
    let InvocationOutput::Owned(value) = output else {
        panic!("owned output")
    };
    assert_eq!(value.downcast::<u32>().unwrap_or_else(|_| panic!("u32")), 11);
    let output = {
        let registry = snapshot(Some(second));
        let MethodLookup::Unique(method) = TypeDescriptor::of::<Counter>().methods_named_in(&registry, "read_async")
        else {
            panic!("generated async method")
        };
        method.invoke_local(&registry, invocation()).unwrap().unwrap()
    };
    let InvocationOutput::Future(future) = output else {
        panic!("local future")
    };
    let mut future = Box::pin(future);
    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    let std::task::Poll::Ready(InvocationOutput::Owned(value)) =
        std::future::Future::poll(future.as_mut(), &mut context)
    else {
        panic!("future resolves independently of the dropped snapshot")
    };
    assert_eq!(value.downcast::<u32>().unwrap_or_else(|_| panic!("u32")), 22);
}

#[test]
fn test_absent_fact_only_and_mismatched_capabilities_preserve_all_inputs() {
    let key = receiver_adapter_key::<Pin<Rc<Counter>>, Local>();
    let wrong_key = CapabilityKey::<u32>::new(*key.id());
    for capability in [
        None,
        Some(CapabilityDescriptor::without_adapter(key)),
        Some(CapabilityDescriptor::with_adapter(wrong_key, 7)),
    ] {
        let seed = snapshot(None);
        let mut builder = RegistrySnapshotBuilder::new();
        for implementation in seed.implementations(TypeDescriptor::of::<Counter>().type_id()) {
            builder.add_impl(implementation, implementation.definition().fragment_identity().clone());
        }
        builder.add_type_capabilities(
            TypeDescriptor::of::<Counter>(),
            capability.into_iter().collect(),
            FragmentIdentity::new("snapshot-call-test", "wrong", 1, 1, "capability", 1),
        );
        let registry = builder.build().unwrap();
        let MethodLookup::Unique(method) = TypeDescriptor::of::<Counter>().methods_named_in(&registry, "read_inputs")
        else {
            panic!("generated method")
        };
        let invocation = Invocation::from_bindings(
            Some(InvocationReceiver::Owned(ReflectedOwned::new(Pin::new(Rc::new(
                Counter { value: 1 },
            ))))),
            [
                InvocationBinding::named("second", InvocationArg::Owned(ReflectedOwned::new(22_u16))),
                InvocationBinding::positional(InvocationArg::Owned(ReflectedOwned::new(11_u8))),
            ],
        );
        let failure = method.invoke_local(&registry, invocation).unwrap().err().unwrap();
        assert!(matches!(
            failure.error().kind(),
            InvocationErrorKind::ReceiverAdapterUnavailable { .. }
        ));
        assert_eq!(failure.recovery().argument_name(0), Some("second"));
        assert_eq!(failure.recovery().argument_name(1), None);
        let (receiver, arguments) = failure.into_recovery().into_parts();
        let mut arguments = arguments.into_vec().into_iter();
        let Some(InvocationArg::Owned(second)) = arguments.next() else {
            panic!("second")
        };
        let Some(InvocationArg::Owned(first)) = arguments.next() else {
            panic!("first")
        };
        assert_eq!(second.downcast::<u16>().unwrap_or_else(|_| panic!("u16")), 22);
        assert_eq!(first.downcast::<u8>().unwrap_or_else(|_| panic!("u8")), 11);
        let Some(InvocationReceiver::Owned(value)) = receiver else {
            panic!("owned receiver")
        };
        assert_eq!(
            value
                .downcast::<Pin<Rc<Counter>>>()
                .unwrap_or_else(|_| panic!("receiver"))
                .value,
            1
        );
    }
}
