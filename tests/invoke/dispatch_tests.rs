// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Regression coverage for dispatch availability and untouched input recovery.

use std::marker::PhantomPinned;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use qubit_reflect::Invocation;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectedOwned;
use qubit_reflect::ReflectedRef;
use qubit_reflect::descriptor::ImplDescriptor;
use qubit_reflect::descriptor::InvocationAdapter;
use qubit_reflect::descriptor::InvocationUnavailableReason;
use qubit_reflect::descriptor::MethodImplementationSource;
use qubit_reflect::descriptor::MethodInstanceDescriptor;
use qubit_reflect::descriptor::MethodLookup;
use qubit_reflect::descriptor::MethodQualifier;
use qubit_reflect::invoke::InvocationArg;
use qubit_reflect::invoke::InvocationBinding;
use qubit_reflect::invoke::InvocationDispatchMode;
use qubit_reflect::invoke::InvocationDispatchReason;
use qubit_reflect::invoke::InvocationFailure;
use qubit_reflect::invoke::InvocationOutput;
use qubit_reflect::invoke::InvocationReceiver;
use qubit_reflect::invoke::InvocationUnavailable;
use qubit_reflect::invoke::PinnedMutAdapter;
use qubit_reflect::invoke::PinnedMutInvocation;
use qubit_reflect::invoke::PinnedMutInvocationFailure;
use qubit_reflect::invoke::PinnedRefAdapter;
use qubit_reflect::invoke::PinnedRefInvocation;
use qubit_reflect::invoke::PinnedRefInvocationFailure;
use qubit_reflect::reflect_impl;
use qubit_reflect::registry::ReflectRegistry;
use qubit_reflect::registry::RegistrySnapshotBuilder;
use qubit_reflect::value::DynamicOwned;
use qubit_reflect::value::Local;
use qubit_reflect::value::ThreadSafe;

struct DropProbe(Arc<AtomicUsize>);
struct BusinessProbe(Arc<AtomicUsize>);
/// Records business execution using a counter owned by each test invocation.
fn record_business(arguments: &[InvocationArg<'_, Local>]) {
    for argument in arguments {
        if let InvocationArg::Owned(value) = argument
            && let Some(probe) = value.downcast_ref::<BusinessProbe>()
        {
            probe.0.fetch_add(1, Ordering::SeqCst);
        }
    }
}
impl Drop for DropProbe {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}
/// Provides an opaque adapter identity without a callable slot.
fn opaque_entry() {}

#[test]
fn test_dispatch_missing_entry_preserves_owned_input() {
    let drops = Arc::new(AtomicUsize::new(0));
    let registry = RegistrySnapshotBuilder::new().build().expect("empty snapshot");
    let input = Invocation::associated([InvocationArg::Owned(ReflectedOwned::new(DropProbe(Arc::clone(&drops))))]);
    let adapter = InvocationAdapter::new(opaque_entry);
    let unavailable = match adapter.invoke_local(&registry, input) {
        Err(error) => error,
        Ok(_) => panic!("opaque adapter must not dispatch"),
    };
    assert_eq!(unavailable.reason(), &InvocationDispatchReason::MissingEntry);
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    let input = unavailable.into_invocation();
    assert_eq!(input.arguments().len(), 1);
    drop(input);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[derive(Reflect)]
struct DispatchFixture;

#[reflect_impl]
impl DispatchFixture {
    /// Supplies a genuine generated declaration for fixture construction.
    fn declaration() {}
}

/// Creates a method instance from a generated declaration without fabricating
/// identity.
fn instance(adapter: Option<&'static InvocationAdapter>) -> MethodInstanceDescriptor {
    let registry = ReflectRegistry::initialize().expect("generated declarations");
    let implementations = registry.implementations(DispatchFixture::type_descriptor().type_id());
    let MethodLookup::Unique(method) =
        ImplDescriptor::lookup_method(implementations, MethodQualifier::Inherent, "declaration")
    else {
        panic!("fixture declaration must exist");
    };
    MethodInstanceDescriptor::new(
        method.declaration(),
        None,
        MethodImplementationSource::Declared,
        adapter,
        if adapter.is_some() {
            Box::new([])
        } else {
            Box::new([
                InvocationUnavailableReason::UnsafeMethod,
                InvocationUnavailableReason::UnsupportedAbi,
            ])
        },
    )
    .expect("valid generated declaration fixture")
}

/// Checks unavailability without adding a Debug requirement to the success
/// type.
fn unavailable<I, R>(
    result: Result<R, InvocationUnavailable<I>>,
    mode: InvocationDispatchMode,
    reason: &InvocationDispatchReason,
) -> I {
    let Err(error) = result else {
        panic!("dispatch must be unavailable")
    };
    assert_eq!(error.mode(), mode);
    assert_eq!(error.reason(), reason);
    error.into_invocation()
}

/// Checks both an owned receiver and caller-ordered mixed bindings are
/// untouched.
macro_rules! ordinary_case {
    ($target:expr, $method:ident, $value_mode:ty, $mode:ident, $reason:expr) => {{
        let business_calls = Arc::new(AtomicUsize::new(0));
        let receiver_drops = Arc::new(AtomicUsize::new(0));
        let argument_drops = Arc::new(AtomicUsize::new(0));
        let input = Invocation::<$value_mode>::from_bindings(
            Some(InvocationReceiver::Owned(DynamicOwned::<$value_mode>::new(DropProbe(
                Arc::clone(&receiver_drops),
            )))),
            [
                InvocationBinding::named(
                    "second",
                    InvocationArg::Owned(DynamicOwned::<$value_mode>::new(DropProbe(Arc::clone(&argument_drops)))),
                ),
                InvocationBinding::positional(InvocationArg::Owned(DynamicOwned::<$value_mode>::new(7_u8))),
                InvocationBinding::positional(InvocationArg::Owned(DynamicOwned::<$value_mode>::new(BusinessProbe(
                    Arc::clone(&business_calls),
                )))),
            ],
        );
        let registry = RegistrySnapshotBuilder::new().build().expect("empty snapshot");
        let input = unavailable(
            $target.$method(&registry, input),
            InvocationDispatchMode::$mode,
            $reason,
        );
        assert_eq!(receiver_drops.load(Ordering::SeqCst), 0);
        assert_eq!(argument_drops.load(Ordering::SeqCst), 0);
        assert_eq!(business_calls.load(Ordering::SeqCst), 0);
        assert!(input.receiver().is_some());
        assert_eq!(input.argument_name(0), Some("second"));
        assert_eq!(input.argument_name(1), None);
        let InvocationArg::Owned(value) = &input.arguments()[1] else {
            panic!("ordered owned argument")
        };
        assert_eq!(value.downcast_ref::<u8>(), Some(&7));
        drop(input);
        assert_eq!(receiver_drops.load(Ordering::SeqCst), 1);
        assert_eq!(argument_drops.load(Ordering::SeqCst), 1);
    }};
}

#[test]
fn test_dispatch_ordinary_modes_missing_slots_and_no_adapter_preserve_inputs() {
    static OPAQUE: InvocationAdapter = InvocationAdapter::new(opaque_entry);
    let missing = InvocationDispatchReason::MissingEntry;
    for high in [false, true] {
        if high {
            let target = instance(Some(&OPAQUE));
            ordinary_case!(target, invoke_local, Local, Local, &missing);
            ordinary_case!(target, invoke_thread_safe, ThreadSafe, ThreadSafe, &missing);
            ordinary_case!(target, invoke_catching_local, Local, CatchingLocal, &missing);
            ordinary_case!(
                target,
                invoke_catching_thread_safe,
                ThreadSafe,
                CatchingThreadSafe,
                &missing
            );
        } else {
            ordinary_case!(OPAQUE, invoke_local, Local, Local, &missing);
            ordinary_case!(OPAQUE, invoke_thread_safe, ThreadSafe, ThreadSafe, &missing);
            ordinary_case!(OPAQUE, invoke_catching_local, Local, CatchingLocal, &missing);
            ordinary_case!(
                OPAQUE,
                invoke_catching_thread_safe,
                ThreadSafe,
                CatchingThreadSafe,
                &missing
            );
        }
    }
    let target = instance(None);
    let reason = InvocationDispatchReason::NoAdapter {
        reasons: target.unavailable_reasons().into(),
    };
    ordinary_case!(target, invoke_local, Local, Local, &reason);
    ordinary_case!(target, invoke_thread_safe, ThreadSafe, ThreadSafe, &reason);
    ordinary_case!(target, invoke_catching_local, Local, CatchingLocal, &reason);
    ordinary_case!(
        target,
        invoke_catching_thread_safe,
        ThreadSafe,
        CatchingThreadSafe,
        &reason
    );
}

/// An immovable receiver whose safe pinned projection remains usable after
/// recovery.
struct Immovable {
    value: AtomicUsize,
    _pin: PhantomPinned,
}

/// Fails visibly if a shared pinned adapter is incorrectly dispatched.
fn pinned_ref_entry<'call>(
    _: &ReflectRegistry,
    input: PinnedRefInvocation<'call, u8, Local>,
) -> Result<InvocationOutput<'call, Local>, PinnedRefInvocationFailure<'call, u8, Local>> {
    record_business(input.arguments());
    panic!("mismatched receiver must not reach business entry")
}
/// Fails visibly if a mutable pinned adapter is incorrectly dispatched.
fn pinned_mut_entry<'call>(
    _: &ReflectRegistry,
    input: PinnedMutInvocation<'call, u8, Local>,
) -> Result<InvocationOutput<'call, Local>, PinnedMutInvocationFailure<'call, u8, Local>> {
    record_business(input.arguments());
    panic!("mismatched receiver must not reach business entry")
}

/// Exercises shared and mutable pins at both dispatch layers and recovers the
/// exact address.
macro_rules! pinned_cases {
    ($target:expr, $reason:expr) => {{
        let registry = RegistrySnapshotBuilder::new().build().expect("empty snapshot");
        let mut receiver = Box::pin(Immovable {
            value: AtomicUsize::new(3),
            _pin: PhantomPinned,
        });
        let business_calls = Arc::new(AtomicUsize::new(0));
        let address = std::ptr::from_ref(receiver.as_ref().get_ref());
        let drops = Arc::new(AtomicUsize::new(0));
        let input = PinnedRefInvocation::from_bindings(
            receiver.as_ref(),
            [
                InvocationBinding::named(
                    "untouched",
                    InvocationArg::Owned(ReflectedOwned::new(DropProbe(Arc::clone(&drops)))),
                ),
                InvocationBinding::positional(InvocationArg::Owned(ReflectedOwned::new(BusinessProbe(Arc::clone(
                    &business_calls,
                ))))),
            ],
        );
        let input = unavailable(
            $target.invoke_pinned_ref_local(&registry, input),
            InvocationDispatchMode::PinnedRefLocal,
            $reason,
        );
        assert_eq!(input.argument_name(0), Some("untouched"));
        assert_eq!(std::ptr::from_ref(input.receiver().get_ref()), address);
        assert_eq!(business_calls.load(Ordering::SeqCst), 0);
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        input.receiver().value.store(5, Ordering::SeqCst);
        drop(input);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        let drops = Arc::new(AtomicUsize::new(0));
        let input = PinnedMutInvocation::from_bindings(
            receiver.as_mut(),
            [
                InvocationBinding::named(
                    "untouched",
                    InvocationArg::Owned(ReflectedOwned::new(DropProbe(Arc::clone(&drops)))),
                ),
                InvocationBinding::positional(InvocationArg::Owned(ReflectedOwned::new(BusinessProbe(Arc::clone(
                    &business_calls,
                ))))),
            ],
        );
        let input = unavailable(
            $target.invoke_pinned_mut_local(&registry, input),
            InvocationDispatchMode::PinnedMutLocal,
            $reason,
        );
        assert_eq!(input.argument_name(0), Some("untouched"));
        assert_eq!(business_calls.load(Ordering::SeqCst), 0);
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        // Validate against a real declaration to expose the original Pin through public
        // recovery.
        let declaration = instance(None);
        let Err(failure) = input.validate(declaration.declaration().identity(), &[]) else {
            panic!("extra argument must fail")
        };
        let mut recovery = failure.into_recovery();
        let recovered_pin = recovery.receiver();
        assert_eq!(std::ptr::from_ref(recovered_pin.as_ref().get_ref()), address);
        recovered_pin.value.store(9, Ordering::SeqCst);
        assert_eq!(business_calls.load(Ordering::SeqCst), 0);
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        drop(recovery);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        assert_eq!(receiver.value.load(Ordering::SeqCst), 9);
    }};
}

#[test]
fn test_dispatch_pinned_modes_missing_slots_type_mismatch_and_no_adapter() {
    static OPAQUE: InvocationAdapter = InvocationAdapter::new(opaque_entry);
    static REF_ENTRY: PinnedRefAdapter<u8, Local> = pinned_ref_entry;
    static MUT_ENTRY: PinnedMutAdapter<u8, Local> = pinned_mut_entry;
    static REF: InvocationAdapter = InvocationAdapter::pinned_ref_local(&REF_ENTRY);
    static MUT: InvocationAdapter = InvocationAdapter::pinned_mut_local(&MUT_ENTRY);
    let missing = InvocationDispatchReason::MissingEntry;
    pinned_cases!(OPAQUE, &missing);
    pinned_cases!(instance(Some(&OPAQUE)), &missing);
    let target = instance(None);
    let no_adapter = InvocationDispatchReason::NoAdapter {
        reasons: target.unavailable_reasons().into(),
    };
    pinned_cases!(target, &no_adapter);
    // Each typed adapter's opposite pin slot is missing, and its exact-T slot
    // rejects !Unpin input.
    let registry = RegistrySnapshotBuilder::new().build().expect("empty snapshot");
    for high in [false, true] {
        let mut receiver = Box::pin(Immovable {
            value: AtomicUsize::new(1),
            _pin: PhantomPinned,
        });
        let business_calls = Arc::new(AtomicUsize::new(0));
        let address = std::ptr::from_ref(receiver.as_ref().get_ref());
        let drops = Arc::new(AtomicUsize::new(0));
        let input = PinnedRefInvocation::new(
            receiver.as_ref(),
            [
                InvocationArg::Owned(ReflectedOwned::new(DropProbe(Arc::clone(&drops)))),
                InvocationArg::Owned(ReflectedOwned::new(BusinessProbe(Arc::clone(&business_calls)))),
            ],
        );
        let result = if high {
            instance(Some(&REF)).invoke_pinned_ref_local(&registry, input)
        } else {
            REF.invoke_pinned_ref_local(&registry, input)
        };
        let input = unavailable(
            result,
            InvocationDispatchMode::PinnedRefLocal,
            &InvocationDispatchReason::PinnedReceiverTypeMismatch,
        );
        assert_eq!(std::ptr::from_ref(input.receiver().get_ref()), address);
        assert_eq!(business_calls.load(Ordering::SeqCst), 0);
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        drop(input);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        let drops = Arc::new(AtomicUsize::new(0));
        let input = PinnedMutInvocation::new(
            receiver.as_mut(),
            [
                InvocationArg::Owned(ReflectedOwned::new(DropProbe(Arc::clone(&drops)))),
                InvocationArg::Owned(ReflectedOwned::new(BusinessProbe(Arc::clone(&business_calls)))),
            ],
        );
        let result = if high {
            instance(Some(&MUT)).invoke_pinned_mut_local(&registry, input)
        } else {
            MUT.invoke_pinned_mut_local(&registry, input)
        };
        let input = unavailable(
            result,
            InvocationDispatchMode::PinnedMutLocal,
            &InvocationDispatchReason::PinnedReceiverTypeMismatch,
        );
        let declaration = instance(None);
        let Err(failure) = input.validate(declaration.declaration().identity(), &[]) else {
            panic!("extra argument")
        };
        let mut recovery = failure.into_recovery();
        assert_eq!(std::ptr::from_ref(recovery.receiver().as_ref().get_ref()), address);
        recovery.receiver().value.store(13, Ordering::SeqCst);
        assert_eq!(business_calls.load(Ordering::SeqCst), 0);
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        drop(recovery);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        assert_eq!(receiver.value.load(Ordering::SeqCst), 13);
    }
}

/// Rejects accidental execution of the ordinary local business entry.
fn local_entry<'call>(
    _: &ReflectRegistry,
    input: Invocation<'call, Local>,
) -> Result<InvocationOutput<'call, Local>, InvocationFailure<'call, Local>> {
    record_business(input.arguments());
    panic!("unavailable catching dispatch must not execute ordinary business entry")
}
/// Rejects accidental execution of the ordinary thread-safe business entry.
fn thread_safe_entry<'call>(
    _: &ReflectRegistry,
    input: Invocation<'call, ThreadSafe>,
) -> Result<InvocationOutput<'call, ThreadSafe>, InvocationFailure<'call, ThreadSafe>> {
    for argument in input.arguments() {
        if let InvocationArg::Owned(value) = argument
            && let Some(probe) = value.downcast_ref::<BusinessProbe>()
        {
            probe.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    panic!("unavailable catching dispatch must not execute ordinary business entry")
}

#[test]
fn test_dispatch_catching_priority_in_both_modes_and_layers() {
    static LOCAL: InvocationAdapter = InvocationAdapter::local(local_entry);
    static SEND: InvocationAdapter = InvocationAdapter::thread_safe(thread_safe_entry);
    static ABORT_LOCAL: InvocationAdapter = InvocationAdapter::local_with_unavailable_catching(local_entry);
    static ABORT_SEND: InvocationAdapter = InvocationAdapter::thread_safe_with_unavailable_catching(thread_safe_entry);
    for (adapter, reason) in [
        (&LOCAL, InvocationDispatchReason::CatchingNotRequested),
        (&ABORT_LOCAL, InvocationDispatchReason::PanicAbort),
    ] {
        ordinary_case!(adapter, invoke_catching_local, Local, CatchingLocal, &reason);
        ordinary_case!(
            instance(Some(adapter)),
            invoke_catching_local,
            Local,
            CatchingLocal,
            &reason
        );
        ordinary_case!(
            adapter,
            invoke_catching_thread_safe,
            ThreadSafe,
            CatchingThreadSafe,
            &InvocationDispatchReason::MissingEntry
        );
        ordinary_case!(
            instance(Some(adapter)),
            invoke_catching_thread_safe,
            ThreadSafe,
            CatchingThreadSafe,
            &InvocationDispatchReason::MissingEntry
        );
    }
    for (adapter, reason) in [
        (&SEND, InvocationDispatchReason::CatchingNotRequested),
        (&ABORT_SEND, InvocationDispatchReason::PanicAbort),
    ] {
        ordinary_case!(
            adapter,
            invoke_catching_thread_safe,
            ThreadSafe,
            CatchingThreadSafe,
            &reason
        );
        ordinary_case!(
            instance(Some(adapter)),
            invoke_catching_thread_safe,
            ThreadSafe,
            CatchingThreadSafe,
            &reason
        );
        ordinary_case!(
            adapter,
            invoke_catching_local,
            Local,
            CatchingLocal,
            &InvocationDispatchReason::MissingEntry
        );
        ordinary_case!(
            instance(Some(adapter)),
            invoke_catching_local,
            Local,
            CatchingLocal,
            &InvocationDispatchReason::MissingEntry
        );
    }
}

#[test]
fn test_dispatch_recovered_input_outlives_registry_snapshot() {
    let drops = Arc::new(AtomicUsize::new(0));
    let input = {
        let registry = RegistrySnapshotBuilder::new().build().expect("empty snapshot");
        let adapter = InvocationAdapter::new(opaque_entry);
        unavailable(
            adapter.invoke_local(
                &registry,
                Invocation::associated([InvocationArg::Owned(ReflectedOwned::new(DropProbe(Arc::clone(&drops))))]),
            ),
            InvocationDispatchMode::Local,
            &InvocationDispatchReason::MissingEntry,
        )
    };
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    drop(input);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    let argument = String::from("borrowed outside snapshot");
    let recovered = {
        let registry = RegistrySnapshotBuilder::new().build().expect("empty snapshot");
        let adapter = InvocationAdapter::new(opaque_entry);
        unavailable(
            adapter.invoke_local(
                &registry,
                Invocation::associated([InvocationArg::Ref(ReflectedRef::new(&argument))]),
            ),
            InvocationDispatchMode::Local,
            &InvocationDispatchReason::MissingEntry,
        )
    };
    let InvocationArg::Ref(value) = &recovered.arguments()[0] else {
        panic!("borrowed argument is preserved")
    };
    assert_eq!(value.downcast_ref::<String>(), Some(&argument));
}
