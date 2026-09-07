// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

//! Isolated receiver resolution never falls back to a failed global registry.
#![cfg(feature = "derive")]

use std::pin::Pin;
use std::rc::Rc;

use qubit_reflect::__private::codegen_v3::inventory::iter;
use qubit_reflect::__private::codegen_v3::registration::RegistrationFragment;
use qubit_reflect::__private::testing::build_registry;
use qubit_reflect::InvocationOutput;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectRegistry;
use qubit_reflect::ReflectedOwned;
use qubit_reflect::capability::CapabilityDescriptor;
use qubit_reflect::descriptor::ImplDescriptor;
use qubit_reflect::descriptor::MethodLookup;
use qubit_reflect::descriptor::MethodQualifier;
use qubit_reflect::identity::FragmentIdentity;
use qubit_reflect::invoke::Invocation;
use qubit_reflect::invoke::InvocationArg;
use qubit_reflect::invoke::InvocationBinding;
use qubit_reflect::invoke::InvocationErrorKind;
use qubit_reflect::invoke::InvocationReceiver;
use qubit_reflect::invoke::ReceiverAdapter;
use qubit_reflect::invoke::receiver_adapter_key;
use qubit_reflect::reflect_impl;
use qubit_reflect::register_type_capabilities;
use qubit_reflect::registry::RegistrySnapshotBuilder;
use qubit_reflect::value::Local;

#[derive(Reflect)]
struct Receiver;

#[reflect_impl]
impl Receiver {
    /// This body must not execute when registry resolution fails.
    fn run(self: Pin<Rc<Self>>, first: u8, second: u16) -> u32 {
        u32::from(first) + u32::from(second)
    }
}

/// Converts only the exact owned explicit receiver.
fn receiver_adapter<'a>(
    receiver: InvocationReceiver<'a, Local>,
) -> Result<Pin<Rc<Receiver>>, InvocationReceiver<'a, Local>> {
    match receiver {
        InvocationReceiver::Owned(value) => value.downcast::<Pin<Rc<Receiver>>>().map_err(InvocationReceiver::Owned),
        receiver => Err(receiver),
    }
}
register_type_capabilities!(Receiver: [
    receiver_adapter_key::<Pin<Rc<Receiver>>, Local>() => receiver_adapter
]);
register_type_capabilities!(String: Clone);
register_type_capabilities!(String: Clone);

/// Copies generated impl facts into the public isolated builder.
fn isolated_builder() -> RegistrySnapshotBuilder {
    let fragments = iter::<RegistrationFragment>
        .into_iter()
        .find_map(|fragment| {
            let registry = build_registry(&[fragment]).ok()?;
            (!registry
                .implementations(Receiver::type_descriptor().type_id())
                .is_empty())
            .then_some(registry)
        })
        .expect("isolated method fragment");
    let mut builder = RegistrySnapshotBuilder::new();
    for implementation in fragments.implementations(Receiver::type_descriptor().type_id()) {
        builder.add_impl(implementation, implementation.definition().fragment_identity().clone());
    }
    builder
}

#[test]
fn test_generated_receiver_resolution_failure_preserves_named_inputs() {
    let registry = isolated_builder().build().unwrap();
    assert!(ReflectRegistry::initialize().is_err());
    let MethodLookup::Unique(method) = ImplDescriptor::lookup_method(
        registry.implementations(Receiver::type_descriptor().type_id()),
        MethodQualifier::Inherent,
        "run",
    ) else {
        panic!("fixture method must exist")
    };
    let receiver = Pin::new(Rc::new(Receiver));
    let invocation = Invocation::from_bindings(
        Some(InvocationReceiver::Owned(ReflectedOwned::new(receiver.clone()))),
        [
            InvocationBinding::named("second", InvocationArg::Owned(ReflectedOwned::new(22_u16))),
            InvocationBinding::positional(InvocationArg::Owned(ReflectedOwned::new(11_u8))),
        ],
    );
    let Some(Err(failure)) = method.invoke_local(&registry, invocation) else {
        panic!("must fail before execution")
    };
    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::ReceiverAdapterUnavailable { .. }
    ));
    assert_eq!(failure.error().method_identity(), method.effective_method().identity());
    assert_eq!(failure.recovery().argument_name(0), Some("second"));
    assert_eq!(failure.recovery().argument_name(1), None);
    let (error, recovery) = failure.into_parts();
    assert_eq!(error.method_identity(), method.effective_method().identity());
    let (recovered, arguments) = recovery.into_parts();
    let Some(InvocationReceiver::Owned(recovered)) = recovered else {
        panic!("owned receiver")
    };
    assert!(recovered.downcast::<Pin<Rc<Receiver>>>().is_ok());
    let mut arguments = arguments.into_vec().into_iter();
    let Some(InvocationArg::Owned(second)) = arguments.next() else {
        panic!("second input")
    };
    let Some(InvocationArg::Owned(first)) = arguments.next() else {
        panic!("first input")
    };
    assert_eq!(second.downcast::<u16>().unwrap_or_else(|_| panic!("u16 input")), 22);
    assert_eq!(first.downcast::<u8>().unwrap_or_else(|_| panic!("u8 input")), 11);
}

#[test]
fn test_explicit_receiver_invocation_succeeds_after_global_initialization_failure() {
    assert!(ReflectRegistry::initialize().is_err());
    let mut builder = isolated_builder();
    builder.add_type_capabilities(
        Receiver::type_descriptor(),
        vec![CapabilityDescriptor::with_adapter(
            receiver_adapter_key::<Pin<Rc<Receiver>>, Local>(),
            receiver_adapter as ReceiverAdapter<Pin<Rc<Receiver>>, Local>,
        )],
        FragmentIdentity::new("isolated", "receiver", 1, 1, "capability", 1),
    );
    let registry = builder.build().unwrap();
    let MethodLookup::Unique(method) = Receiver::type_descriptor().methods_named_in(&registry, "run") else {
        panic!("generated method in local snapshot")
    };
    let invocation = Invocation::owned(
        ReflectedOwned::new(Pin::new(Rc::new(Receiver))),
        [
            InvocationArg::Owned(ReflectedOwned::new(11_u8)),
            InvocationArg::Owned(ReflectedOwned::new(22_u16)),
        ],
    );
    let output = method.invoke_local(&registry, invocation).unwrap().unwrap();
    let InvocationOutput::Owned(value) = output else {
        panic!("owned result")
    };
    assert_eq!(value.downcast::<u32>().unwrap_or_else(|_| panic!("u32")), 33);
    assert!(ReflectRegistry::initialize().is_err());
}
