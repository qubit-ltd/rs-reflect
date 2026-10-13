// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::future::Future;
use std::pin::Pin;
use std::sync::LazyLock;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;

use qubit_reflect::descriptor::CatchingAvailability;
use qubit_reflect::descriptor::ImplDefinitionDescriptor;
use qubit_reflect::descriptor::ImplKind;
use qubit_reflect::descriptor::InvocationAdapter as DescriptorInvocationAdapter;
use qubit_reflect::descriptor::MethodDeclarationOwner;
use qubit_reflect::descriptor::MethodDescriptor;
use qubit_reflect::descriptor::MethodImplementationSource;
use qubit_reflect::descriptor::MethodInstanceDescriptor;
use qubit_reflect::descriptor::MethodQualifiers;
use qubit_reflect::descriptor::MethodVisibility;
use qubit_reflect::descriptor::ParameterDescriptor;
use qubit_reflect::descriptor::ParameterPassingMode;
use qubit_reflect::descriptor::ParameterPatternDescriptor;
use qubit_reflect::descriptor::ReceiverDescriptor;
use qubit_reflect::descriptor::ReturnDescriptor;
use qubit_reflect::descriptor::ReturnKind;
use qubit_reflect::expression::ExpressionName;
use qubit_reflect::expression::FunctionAbi;
use qubit_reflect::expression::GenericDefinitionDescriptor;
use qubit_reflect::expression::TypeExpression;
use qubit_reflect::identity::FragmentIdentity;
use qubit_reflect::identity::MemberId;
use qubit_reflect::identity::Visibility;
use qubit_reflect::invoke::ArgumentExpectation;
use qubit_reflect::invoke::BorrowOrigin;
use qubit_reflect::invoke::CatchingInvocationAdapter;
use qubit_reflect::invoke::CatchingInvocationResult;
use qubit_reflect::invoke::Invocation;
use qubit_reflect::invoke::InvocationAdapter;
use qubit_reflect::invoke::InvocationArg;
use qubit_reflect::invoke::InvocationBinding;
use qubit_reflect::invoke::InvocationFailure;
use qubit_reflect::invoke::InvocationOutput;
use qubit_reflect::invoke::PinnedMutAdapter;
use qubit_reflect::invoke::PinnedMutInvocation;
use qubit_reflect::invoke::PinnedMutInvocationFailure;
use qubit_reflect::invoke::PinnedRefAdapter;
use qubit_reflect::invoke::PinnedRefInvocation;
use qubit_reflect::invoke::PinnedRefInvocationFailure;
use qubit_reflect::invoke::ReceiverExpectation;
use qubit_reflect::invoke::ReflectedFuture;
use qubit_reflect::registry::ReflectRegistry;
use qubit_reflect::value::DynamicMut;
use qubit_reflect::value::DynamicOwned;
use qubit_reflect::value::DynamicRef;
use qubit_reflect::value::Local;
use qubit_reflect::value::ThreadSafe;

fn opaque_entry_point() {}

fn validation_identity() -> MemberId {
    MemberId::new(
        "coverage::ValidationTarget",
        "validate",
        0,
        FragmentIdentity::new("coverage", "coverage::ValidationTarget", 1, 0, "validate", 0),
    )
}

fn local_entry<'call>(
    _: &ReflectRegistry,
    _: Invocation<'call, Local>,
) -> Result<InvocationOutput<'call, Local>, InvocationFailure<'call, Local>> {
    Ok(InvocationOutput::Unit)
}

fn thread_safe_entry<'call>(
    _: &ReflectRegistry,
    _: Invocation<'call, ThreadSafe>,
) -> Result<InvocationOutput<'call, ThreadSafe>, InvocationFailure<'call, ThreadSafe>> {
    Ok(InvocationOutput::Unit)
}

fn local_catching_entry<'call>(
    _: &ReflectRegistry,
    _: Invocation<'call, Local>,
) -> CatchingInvocationResult<'call, Local> {
    Ok(Ok(InvocationOutput::Unit))
}

fn thread_safe_catching_entry<'call>(
    _: &ReflectRegistry,
    _: Invocation<'call, ThreadSafe>,
) -> CatchingInvocationResult<'call, ThreadSafe> {
    Ok(Ok(InvocationOutput::Unit))
}

fn pinned_ref_entry<'call>(
    _: &ReflectRegistry,
    _: PinnedRefInvocation<'call, u8, Local>,
) -> Result<InvocationOutput<'call, Local>, PinnedRefInvocationFailure<'call, u8, Local>> {
    Ok(InvocationOutput::Unit)
}

fn pinned_mut_entry<'call>(
    _: &ReflectRegistry,
    _: PinnedMutInvocation<'call, u8, Local>,
) -> Result<InvocationOutput<'call, Local>, PinnedMutInvocationFailure<'call, u8, Local>> {
    Ok(InvocationOutput::Unit)
}

fn pinned_mut_validating_entry<'call>(
    _: &ReflectRegistry,
    invocation: PinnedMutInvocation<'call, u8, Local>,
) -> Result<InvocationOutput<'call, Local>, PinnedMutInvocationFailure<'call, u8, Local>> {
    let validated = invocation.validate(&identity(7), &[ArgumentExpectation::owned::<String>()])?;
    let (_, arguments) = validated.into_parts();
    assert_eq!(arguments.len(), 1);
    Ok(InvocationOutput::Unit)
}

fn owned_receiver_entry<'call>(
    _: &ReflectRegistry,
    invocation: Invocation<'call, Local>,
) -> Result<InvocationOutput<'call, Local>, InvocationFailure<'call, Local>> {
    let validated = invocation.validate(&identity(8), ReceiverExpectation::owned::<u16>(), &[])?;
    let (Some(receiver), _) = validated.into_parts() else {
        unreachable!("owned receiver validation returns a receiver")
    };
    assert_eq!(receiver.type_id(), std::any::TypeId::of::<u16>());
    Ok(InvocationOutput::Owned(DynamicOwned::<Local>::new(99_u8)))
}

fn str_return_entry<'call>(
    _: &ReflectRegistry,
    invocation: Invocation<'call, Local>,
) -> Result<InvocationOutput<'call, Local>, InvocationFailure<'call, Local>> {
    let validated = invocation.validate(
        &identity(9),
        ReceiverExpectation::none(),
        &[ArgumentExpectation::borrowed::<str>()],
    )?;
    let (_, arguments) = validated.into_parts();
    let [InvocationArg::Ref(value)] = arguments.as_ref() else {
        unreachable!("the validated borrowed str remains a dynamic reference")
    };
    Ok(InvocationOutput::Ref {
        value: value.clone(),
        origins: Box::new([BorrowOrigin::Parameter(0)]),
    })
}

static PINNED_REF_ENTRY: PinnedRefAdapter<u8, Local> = pinned_ref_entry;
static PINNED_MUT_ENTRY: PinnedMutAdapter<u8, Local> = pinned_mut_entry;
static PINNED_MUT_VALIDATING_ENTRY: PinnedMutAdapter<u8, Local> = pinned_mut_validating_entry;

static GENERIC_DEFINITION: LazyLock<GenericDefinitionDescriptor> =
    LazyLock::new(|| GenericDefinitionDescriptor::new([], []));
static IMPL_DEFINITION: std::sync::LazyLock<ImplDefinitionDescriptor> = std::sync::LazyLock::new(|| {
    ImplDefinitionDescriptor::new(
        FragmentIdentity::new("coverage", "invocation_coverage_tests", 1, 0, "impl", 0),
        TypeExpression::Never,
        ImplKind::Inherent,
        None,
        &GENERIC_DEFINITION,
    )
    .expect("the synthetic inherent implementation is valid")
});
static METHOD_BODY_CALLS: AtomicUsize = AtomicUsize::new(0);

struct ValidationReceiver;

fn validating_entry<'call>(
    _: &ReflectRegistry,
    invocation: Invocation<'call, Local>,
) -> Result<InvocationOutput<'call, Local>, InvocationFailure<'call, Local>> {
    let validated = invocation.validate(
        &identity(5),
        ReceiverExpectation::borrowed::<ValidationReceiver>(),
        &[ArgumentExpectation::owned::<String>()],
    )?;
    let (_, arguments) = validated.into_parts();
    METHOD_BODY_CALLS.fetch_add(1, Ordering::SeqCst);
    assert_eq!(arguments.len(), 1);
    Ok(InvocationOutput::Unit)
}

#[derive(Debug)]
struct PinnedFutureReceiver {
    value: u8,
}

fn pinned_future_entry<'call>(
    _: &ReflectRegistry,
    invocation: PinnedRefInvocation<'call, PinnedFutureReceiver, Local>,
) -> Result<InvocationOutput<'call, Local>, PinnedRefInvocationFailure<'call, PinnedFutureReceiver, Local>> {
    let receiver = invocation.receiver();
    Ok(InvocationOutput::Future(ReflectedFuture::<Local>::new(async move {
        InvocationOutput::Owned(DynamicOwned::<Local>::new(receiver.get_ref().value))
    })))
}

static PINNED_FUTURE_ENTRY: PinnedRefAdapter<PinnedFutureReceiver, Local> = pinned_future_entry;

fn identity(index: usize) -> MemberId {
    MemberId::new(
        "coverage::InvocationTarget",
        "method",
        index,
        FragmentIdentity::new(
            "coverage",
            "invocation_coverage_tests",
            1,
            index as u32,
            "method",
            index as u64,
        ),
    )
}

fn validation_method(adapter: &'static DescriptorInvocationAdapter) -> MethodInstanceDescriptor {
    let method = Box::leak(Box::new(
        MethodDescriptor::builder(
            identity(5),
            "validate",
            "validate",
            MethodDeclarationOwner::Impl(&IMPL_DEFINITION),
        )
        .visibility(MethodVisibility::Declared(Visibility::Public))
        .receiver(Some(ReceiverDescriptor::Shared))
        .parameters(vec![ParameterDescriptor::new(
            0,
            Some("value"),
            ParameterPatternDescriptor::Identifier,
            ParameterPassingMode::Owned,
            TypeExpression::Never,
            None,
        )])
        .build(),
    ));
    MethodInstanceDescriptor::new(
        method,
        None,
        MethodImplementationSource::Declared,
        Some(adapter),
        Box::new([]),
    )
    .expect("the declared method and local adapter are compatible")
}

fn pinned_future_method(adapter: &'static DescriptorInvocationAdapter) -> MethodInstanceDescriptor {
    let method = Box::leak(Box::new(
        MethodDescriptor::builder(
            identity(6),
            "pinned_future",
            "pinned_future",
            MethodDeclarationOwner::Impl(&IMPL_DEFINITION),
        )
        .visibility(MethodVisibility::Declared(Visibility::Public))
        .receiver(Some(ReceiverDescriptor::Explicit("Pin<&Self>")))
        .qualifiers(MethodQualifiers::new(true, false, false, None, false))
        .build(),
    ));
    MethodInstanceDescriptor::new(
        method,
        None,
        MethodImplementationSource::Declared,
        Some(adapter),
        Box::new([]),
    )
    .expect("the pinned future method declaration is valid")
}

fn pinned_mut_method(adapter: &'static DescriptorInvocationAdapter) -> MethodInstanceDescriptor {
    let method = Box::leak(Box::new(
        MethodDescriptor::builder(
            identity(7),
            "pinned_mut",
            "pinned_mut",
            MethodDeclarationOwner::Impl(&IMPL_DEFINITION),
        )
        .visibility(MethodVisibility::Declared(Visibility::Public))
        .receiver(Some(ReceiverDescriptor::Explicit("Pin<&mut Self>")))
        .parameters(vec![ParameterDescriptor::new(
            0,
            Some("value"),
            ParameterPatternDescriptor::Identifier,
            ParameterPassingMode::Owned,
            TypeExpression::Never,
            None,
        )])
        .build(),
    ));
    MethodInstanceDescriptor::new(
        method,
        None,
        MethodImplementationSource::Declared,
        Some(adapter),
        Box::new([]),
    )
    .expect("the pinned mutable method declaration is valid")
}

fn private_pinned_mut_method(adapter: &'static DescriptorInvocationAdapter) -> MethodInstanceDescriptor {
    let method = Box::leak(Box::new(
        MethodDescriptor::builder(
            identity(10),
            "private_pinned_mut",
            "private_pinned_mut",
            MethodDeclarationOwner::Impl(&IMPL_DEFINITION),
        )
        .visibility(MethodVisibility::Declared(Visibility::Private))
        .receiver(Some(ReceiverDescriptor::Explicit("Pin<&mut Self>")))
        .parameters(vec![ParameterDescriptor::new(
            0,
            Some("value"),
            ParameterPatternDescriptor::Identifier,
            ParameterPassingMode::Owned,
            TypeExpression::Never,
            None,
        )])
        .build(),
    ));
    MethodInstanceDescriptor::new(
        method,
        None,
        MethodImplementationSource::Declared,
        Some(adapter),
        Box::new([]),
    )
    .expect("the private pinned mutable method declaration is valid")
}

fn owned_receiver_method(adapter: &'static DescriptorInvocationAdapter) -> MethodInstanceDescriptor {
    let method = Box::leak(Box::new(
        MethodDescriptor::builder(
            identity(8),
            "owned_receiver",
            "owned_receiver",
            MethodDeclarationOwner::Impl(&IMPL_DEFINITION),
        )
        .visibility(MethodVisibility::Declared(Visibility::Public))
        .receiver(Some(ReceiverDescriptor::Owned))
        .build(),
    ));
    MethodInstanceDescriptor::new(
        method,
        None,
        MethodImplementationSource::Declared,
        Some(adapter),
        Box::new([]),
    )
    .expect("the owned receiver method declaration is valid")
}

fn str_return_method(adapter: &'static DescriptorInvocationAdapter) -> MethodInstanceDescriptor {
    let method = Box::leak(Box::new(
        MethodDescriptor::builder(
            identity(9),
            "str_return",
            "str_return",
            MethodDeclarationOwner::Impl(&IMPL_DEFINITION),
        )
        .visibility(MethodVisibility::Declared(Visibility::Public))
        .parameters(vec![ParameterDescriptor::new(
            0,
            Some("text"),
            ParameterPatternDescriptor::Identifier,
            ParameterPassingMode::SharedBorrow,
            TypeExpression::Never,
            None,
        )])
        .return_value(ReturnDescriptor::new(
            ReturnKind::Reference,
            Some(TypeExpression::Never),
            None,
        ))
        .build(),
    ));
    MethodInstanceDescriptor::new(
        method,
        None,
        MethodImplementationSource::Declared,
        Some(adapter),
        Box::new([]),
    )
    .expect("the borrowed str return method declaration is valid")
}

#[test]
fn test_reflected_future_forwards_pending_and_ready_polls_in_both_modes() {
    let mut wake_count = 0;
    let mut local_future = ReflectedFuture::<Local>::new(std::future::poll_fn(|context| {
        wake_count += 1;
        if wake_count == 1 {
            context.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(InvocationOutput::Owned(DynamicOwned::<Local>::new(21_u8)))
        }
    }));
    let mut context = Context::from_waker(Waker::noop());
    assert!(matches!(Pin::new(&mut local_future).poll(&mut context), Poll::Pending));
    let Poll::Ready(InvocationOutput::Owned(value)) = Pin::new(&mut local_future).poll(&mut context) else {
        panic!("the local future must forward its ready output after Pending");
    };
    assert_eq!(value.downcast_ref::<u8>(), Some(&21));

    let mut thread_safe_future =
        ReflectedFuture::<ThreadSafe>::new(async { InvocationOutput::Owned(DynamicOwned::<ThreadSafe>::new(34_u16)) });
    let Poll::Ready(InvocationOutput::Owned(value)) = Pin::new(&mut thread_safe_future).poll(&mut context) else {
        panic!("the thread-safe future must forward its ready output");
    };
    assert_eq!(value.downcast_ref::<u16>(), Some(&34));
}

#[test]
fn test_argument_validation_checks_local_and_thread_safe_erased_type_ids() {
    let identity = validation_identity();

    let local_owned = Invocation::<Local>::associated([InvocationArg::Owned(DynamicOwned::<Local>::new(21_u32))])
        .validate(
            &identity,
            ReceiverExpectation::none(),
            &[ArgumentExpectation::owned::<u32>()],
        )
        .expect("the exact owned type must validate in local mode");
    assert_eq!(local_owned.arguments().len(), 1);

    let shared_owned =
        Invocation::<ThreadSafe>::associated([InvocationArg::Owned(DynamicOwned::<ThreadSafe>::new(34_u64))])
            .validate(
                &identity,
                ReceiverExpectation::none(),
                &[ArgumentExpectation::owned::<u64>()],
            )
            .expect("the exact owned type must validate in thread-safe mode");
    assert_eq!(shared_owned.arguments().len(), 1);

    let local_text = "local reflected text";
    let local_ref = Invocation::<Local>::associated([InvocationArg::Ref(DynamicRef::<Local>::new_str(local_text))])
        .validate(
            &identity,
            ReceiverExpectation::none(),
            &[ArgumentExpectation::borrowed::<str>()],
        )
        .expect("the dedicated local str representation must validate as str");
    assert_eq!(local_ref.arguments().len(), 1);

    let shared_text = "thread-safe reflected text";
    let shared_ref =
        Invocation::<ThreadSafe>::associated([InvocationArg::Ref(DynamicRef::<ThreadSafe>::new_str(shared_text))])
            .validate(
                &identity,
                ReceiverExpectation::none(),
                &[ArgumentExpectation::borrowed::<str>()],
            )
            .expect("the dedicated thread-safe str representation must validate as str");
    assert_eq!(shared_ref.arguments().len(), 1);

    let mut local_text = String::from("local mut str");
    let local_mut =
        Invocation::<Local>::associated([InvocationArg::Mut(DynamicMut::<Local>::new_str_mut(&mut local_text))])
            .validate(
                &identity,
                ReceiverExpectation::none(),
                &[ArgumentExpectation::borrowed_mut::<str>()],
            )
            .expect("the local mutable str representation must validate as str");
    assert_eq!(local_mut.arguments().len(), 1);

    let mut shared_text = String::from("thread-safe mut str");
    let shared_mut = Invocation::<ThreadSafe>::associated([InvocationArg::Mut(DynamicMut::<ThreadSafe>::new_str_mut(
        &mut shared_text,
    ))])
    .validate(
        &identity,
        ReceiverExpectation::none(),
        &[ArgumentExpectation::borrowed_mut::<str>()],
    )
    .expect("the thread-safe mutable str representation must validate as str");
    assert_eq!(shared_mut.arguments().len(), 1);
}

#[test]
fn test_method_instance_rejects_bad_receiver_and_arguments_without_consuming_inputs() {
    static ADAPTER: DescriptorInvocationAdapter = DescriptorInvocationAdapter::local(validating_entry);
    let method = validation_method(&ADAPTER);
    let registry = ReflectRegistry::initialize().expect("the registry must initialize");
    METHOD_BODY_CALLS.store(0, Ordering::SeqCst);

    let wrong_receiver = 77_u16;
    let failure = match method.invoke(
        registry,
        Invocation::receiver_ref(&wrong_receiver)
            .arg(13_u8)
            .arg(String::from("preserved after receiver failure")),
    ) {
        Ok(_) => panic!("the exact receiver type is checked before entering the method body"),
        Err(failure) => failure,
    };
    assert!(matches!(
        failure.error().kind(),
        qubit_reflect::invoke::InvocationErrorKind::ReceiverTypeMismatch { .. }
    ));
    let recovery = failure.recovery();
    let Some(qubit_reflect::invoke::InvocationReceiver::Ref(receiver)) = recovery.receiver() else {
        panic!("the original shared receiver must remain available for retry");
    };
    assert_eq!(receiver.downcast_ref::<u16>(), Some(&77));
    assert_eq!(recovery.arguments().len(), 2);
    let [InvocationArg::Owned(first), InvocationArg::Owned(second)] = recovery.arguments() else {
        panic!("recovery retains both original owned arguments in caller order");
    };
    assert_eq!(first.downcast_ref::<u8>(), Some(&13));
    assert_eq!(
        second.downcast_ref::<String>().map(String::as_str),
        Some("preserved after receiver failure")
    );

    let target = ValidationReceiver;
    let failure = match method.invoke(registry, Invocation::receiver_ref(&target).arg(29_u16)) {
        Ok(_) => panic!("the exact parameter type is checked before entering the method body"),
        Err(failure) => failure,
    };
    assert!(matches!(
        failure.error().kind(),
        qubit_reflect::invoke::InvocationErrorKind::ArgumentTypeMismatch { .. }
    ));
    assert_eq!(failure.recovery().arguments().len(), 1);
    let [InvocationArg::Owned(argument)] = failure.recovery().arguments() else {
        panic!("the original argument remains available for retry");
    };
    assert_eq!(argument.downcast_ref::<u16>(), Some(&29));
    assert_eq!(
        METHOD_BODY_CALLS.load(Ordering::SeqCst),
        0,
        "invalid inputs must never enter the method body"
    );
}

#[test]
fn test_method_instance_pinned_ref_adapter_returns_a_pollable_borrow_scoped_future() {
    static ADAPTER: DescriptorInvocationAdapter = DescriptorInvocationAdapter::pinned_ref_local(&PINNED_FUTURE_ENTRY);
    let method = pinned_future_method(&ADAPTER);
    let registry = ReflectRegistry::initialize().expect("the registry must initialize");
    let receiver = Box::pin(PinnedFutureReceiver { value: 42 });

    let InvocationOutput::Future(mut future) = method
        .invoke_pinned_ref(registry, PinnedRefInvocation::new(receiver.as_ref(), []))
        .expect("the public pinned method adapter must be callable")
    else {
        panic!("the async adapter returns a reflected future");
    };
    let mut context = Context::from_waker(Waker::noop());
    let Poll::Ready(InvocationOutput::Owned(value)) = Pin::new(&mut future).poll(&mut context) else {
        panic!("the returned future must preserve the receiver borrow and complete");
    };
    assert_eq!(value.downcast_ref::<u8>(), Some(&42));
}

#[test]
fn test_invocation_adapter_dispatch_reports_absent_and_configured_catching_entries() {
    let registry = ReflectRegistry::initialize().expect("the registry must initialize");
    let opaque = DescriptorInvocationAdapter::new(opaque_entry_point);
    assert_eq!(opaque.catching_availability(), CatchingAvailability::NotRequested);

    let unavailable_local = match opaque.invoke_local(registry, Invocation::<Local>::associated([])) {
        Ok(_) => panic!("an opaque token has no callable local entry"),
        Err(unavailable) => unavailable,
    };
    assert_eq!(
        unavailable_local.reason(),
        &qubit_reflect::invoke::InvocationDispatchReason::MissingEntry
    );
    assert!(unavailable_local.into_invocation().arguments().is_empty());

    let unavailable_thread_safe = match opaque.invoke_thread_safe(registry, Invocation::<ThreadSafe>::associated([])) {
        Ok(_) => panic!("an opaque token has no callable thread-safe entry"),
        Err(unavailable) => unavailable,
    };
    assert_eq!(
        unavailable_thread_safe.reason(),
        &qubit_reflect::invoke::InvocationDispatchReason::MissingEntry
    );

    let local_without_catching = DescriptorInvocationAdapter::local(local_entry as InvocationAdapter<Local>);
    let no_catching = match local_without_catching.invoke_catching_local(registry, Invocation::<Local>::associated([]))
    {
        Ok(_) => panic!("ordinary local adapters must not imply a catching entry"),
        Err(unavailable) => unavailable,
    };
    assert_eq!(
        no_catching.reason(),
        &qubit_reflect::invoke::InvocationDispatchReason::CatchingNotRequested
    );

    let thread_safe_without_catching =
        DescriptorInvocationAdapter::thread_safe(thread_safe_entry as InvocationAdapter<ThreadSafe>);
    let no_thread_safe_catching = match thread_safe_without_catching
        .invoke_catching_thread_safe(registry, Invocation::<ThreadSafe>::associated([]))
    {
        Ok(_) => panic!("ordinary thread-safe adapters must not imply a catching entry"),
        Err(unavailable) => unavailable,
    };
    assert_eq!(
        no_thread_safe_catching.reason(),
        &qubit_reflect::invoke::InvocationDispatchReason::CatchingNotRequested
    );

    let local_catching = DescriptorInvocationAdapter::local_with_catching(
        local_entry as InvocationAdapter<Local>,
        local_catching_entry as CatchingInvocationAdapter<Local>,
    );
    assert_eq!(local_catching.catching_availability(), CatchingAvailability::Available);
    assert!(matches!(
        local_catching
            .invoke_catching_local(registry, Invocation::<Local>::associated([]))
            .expect("the catching local entry exists")
            .expect("the adapter accepts the empty invocation"),
        Ok(InvocationOutput::Unit)
    ));

    let thread_safe_catching = DescriptorInvocationAdapter::thread_safe_with_catching(
        thread_safe_entry as InvocationAdapter<ThreadSafe>,
        thread_safe_catching_entry as CatchingInvocationAdapter<ThreadSafe>,
    );
    assert!(matches!(
        thread_safe_catching
            .invoke_catching_thread_safe(registry, Invocation::<ThreadSafe>::associated([]))
            .expect("the catching thread-safe entry exists")
            .expect("the adapter accepts the empty invocation"),
        Ok(InvocationOutput::Unit)
    ));
}

#[test]
fn test_pinned_adapter_dispatch_checks_exact_receiver_type_and_runs_matching_entries() {
    let registry = ReflectRegistry::initialize().expect("the registry must initialize");
    let ref_adapter = DescriptorInvocationAdapter::pinned_ref_local(&PINNED_REF_ENTRY);
    let pinned_u8 = Box::pin(7_u8);
    assert!(matches!(
        ref_adapter
            .invoke_pinned_ref_local(registry, PinnedRefInvocation::<u8, Local>::new(pinned_u8.as_ref(), []),)
            .expect("the ref-pinned entry exists")
            .expect("the matching receiver is accepted"),
        InvocationOutput::Unit
    ));

    let pinned_u16 = Box::pin(9_u16);
    let mismatch = match ref_adapter.invoke_pinned_ref_local(
        registry,
        PinnedRefInvocation::<u16, Local>::new(pinned_u16.as_ref(), []),
    ) {
        Ok(_) => panic!("a pinned receiver of a different type must not reach the adapter"),
        Err(unavailable) => unavailable,
    };
    assert_eq!(
        mismatch.reason(),
        &qubit_reflect::invoke::InvocationDispatchReason::PinnedReceiverTypeMismatch
    );
    assert_eq!(*mismatch.into_invocation().receiver(), 9);

    let mut_adapter = DescriptorInvocationAdapter::pinned_mut_local(&PINNED_MUT_ENTRY);
    let mut pinned_mut_u8 = Box::pin(11_u8);
    assert!(matches!(
        mut_adapter
            .invoke_pinned_mut_local(
                registry,
                PinnedMutInvocation::<u8, Local>::new(pinned_mut_u8.as_mut(), [])
            )
            .expect("the mut-pinned entry exists")
            .expect("the matching receiver is accepted"),
        InvocationOutput::Unit
    ));

    let mut pinned_mut_u16 = Box::pin(13_u16);
    let mismatch = match mut_adapter.invoke_pinned_mut_local(
        registry,
        PinnedMutInvocation::<u16, Local>::new(pinned_mut_u16.as_mut(), []),
    ) {
        Ok(_) => panic!("a pinned mutable receiver of a different type must not reach the adapter"),
        Err(unavailable) => unavailable,
    };
    assert_eq!(
        mismatch.reason(),
        &qubit_reflect::invoke::InvocationDispatchReason::PinnedReceiverTypeMismatch
    );
    let recovered = mismatch.into_invocation();
    assert!(recovered.arguments().is_empty());
    drop(recovered);
    assert_eq!(*pinned_mut_u16.as_ref().get_ref(), 13);
}

#[test]
fn test_method_qualifiers_preserve_every_modifier_and_custom_abi() {
    let qualifiers = MethodQualifiers::new(true, true, true, Some(FunctionAbi::C), true);
    assert!(qualifiers.is_async());
    assert!(qualifiers.is_unsafe());
    assert!(qualifiers.is_const());
    assert_eq!(qualifiers.abi(), Some(&FunctionAbi::C));
    assert!(qualifiers.is_variadic());

    let custom_abi = FunctionAbi::Other(ExpressionName::new("vectorcall").expect("ABI name must be valid"));
    let custom = MethodQualifiers::new(false, false, false, Some(custom_abi.clone()), false);
    assert_eq!(custom.abi(), Some(&custom_abi));
    assert!(!custom.is_async());
    assert!(!custom.is_unsafe());
    assert!(!custom.is_const());
    assert!(!custom.is_variadic());
}

#[test]
fn test_method_instance_pinned_mut_success_and_failure_recover_inputs() {
    static ADAPTER: DescriptorInvocationAdapter =
        DescriptorInvocationAdapter::pinned_mut_local(&PINNED_MUT_VALIDATING_ENTRY);
    let method = pinned_mut_method(&ADAPTER);
    let registry = ReflectRegistry::initialize().expect("the registry must initialize");

    let mut receiver = Box::pin(21_u8);
    assert!(matches!(
        method.invoke_pinned_mut(
            registry,
            Invocation::receiver_pin_mut(receiver.as_mut()).arg(String::from("accepted")),
        ),
        Ok(InvocationOutput::Unit)
    ));

    let Err(failure) = method.invoke_pinned_mut(registry, Invocation::receiver_pin_mut(receiver.as_mut()).arg(13_u16))
    else {
        panic!("the adapter rejects the wrong argument type before execution")
    };
    assert!(matches!(
        failure.error().kind(),
        qubit_reflect::invoke::InvocationErrorKind::ArgumentTypeMismatch { .. }
    ));
    let mut recovery = failure.into_recovery();
    assert_eq!(*recovery.receiver().as_ref().get_ref(), 21);
    assert_eq!(recovery.arguments().len(), 1);
    let [InvocationArg::Owned(argument)] = recovery.arguments() else {
        panic!("the owned argument remains available for retry")
    };
    assert_eq!(argument.downcast_ref::<u16>(), Some(&13));
}

#[test]
fn test_pinned_mut_builders_and_private_visibility_unsafe_entry() {
    static ADAPTER: DescriptorInvocationAdapter =
        DescriptorInvocationAdapter::pinned_mut_local(&PINNED_MUT_VALIDATING_ENTRY);
    let method = private_pinned_mut_method(&ADAPTER);
    let registry = ReflectRegistry::initialize().expect("the registry must initialize");

    let mut receiver = Box::pin(31_u8);
    let shared = 5_u16;
    let mut mutable = 8_u32;
    let invocation = PinnedMutInvocation::<u8, Local>::new(receiver.as_mut(), [])
        .arg_ref(&shared)
        .named_arg("value", String::from("allowed"))
        .arg_mut(&mut mutable);
    assert_eq!(invocation.arguments().len(), 3);
    assert_eq!(invocation.argument_name(1), Some("value"));
    assert_eq!(invocation.argument_name(3), None);

    let failure = match method.invoke_pinned_mut(registry, invocation) {
        Err(failure) => failure,
        Ok(_) => panic!("safe pinned invocation rejects a private method"),
    };
    assert!(matches!(
        failure.error().kind(),
        qubit_reflect::invoke::InvocationErrorKind::VisibilityDenied
    ));
    let mut recovery = failure.into_recovery();
    assert_eq!(*recovery.receiver().as_ref().get_ref(), 31);
    assert_eq!(recovery.argument_name(1), Some("value"));
    drop(recovery);

    let wrong_type = PinnedMutInvocation::<u8, Local>::new(receiver.as_mut(), []).named_arg("value", 12_u16);
    // SAFETY: This test deliberately exercises the documented visibility bypass
    // for a private fixture method whose adapter only validates and returns.
    let Err(failure) = (unsafe { method.invoke_pinned_mut_unsafe(registry, wrong_type) }) else {
        panic!("unsafe invocation still validates argument types")
    };
    assert!(matches!(
        failure.error().kind(),
        qubit_reflect::invoke::InvocationErrorKind::ArgumentTypeMismatch { .. }
    ));
    let mut recovery = failure.into_recovery();
    assert_eq!(*recovery.receiver().as_ref().get_ref(), 31);
    assert_eq!(recovery.argument_name(0), Some("value"));
}

#[test]
fn test_pinned_invocation_binding_builders_preserve_borrow_modes() {
    let shared_receiver = Box::pin(41_u8);
    let shared = 6_u16;
    let mut mutable = 9_u32;
    let pinned_ref = PinnedRefInvocation::<u8, Local>::new(shared_receiver.as_ref(), [])
        .arg_ref(&shared)
        .named_arg("named", 17_u8)
        .arg_mut(&mut mutable);
    assert_eq!(pinned_ref.arguments().len(), 3);
    assert_eq!(pinned_ref.argument_name(1), Some("named"));
    assert_eq!(pinned_ref.argument_name(3), None);

    let mut mut_receiver = Box::pin(42_u8);
    let binding = InvocationBinding::named("bound", InvocationArg::Owned(DynamicOwned::<Local>::new(23_u32)));
    let pinned_mut = PinnedMutInvocation::<u8, Local>::from_bindings(mut_receiver.as_mut(), [binding]);
    assert_eq!(pinned_mut.arguments().len(), 1);
    assert_eq!(pinned_mut.argument_name(0), Some("bound"));
}

#[test]
fn test_method_instance_owned_receiver_and_missing_adapter_entry() {
    static ADAPTER: DescriptorInvocationAdapter = DescriptorInvocationAdapter::local(owned_receiver_entry);
    let method = owned_receiver_method(&ADAPTER);
    let registry = ReflectRegistry::initialize().expect("the registry must initialize");
    let output = method
        .invoke(registry, Invocation::receiver_owned(42_u16))
        .expect("the exact owned receiver reaches its adapter");
    let InvocationOutput::Owned(output) = output else {
        panic!("the fixture returns an owned value")
    };
    assert_eq!(output.downcast_ref::<u8>(), Some(&99));

    static OPAQUE: DescriptorInvocationAdapter = DescriptorInvocationAdapter::new(opaque_entry_point);
    let method = owned_receiver_method(&OPAQUE);
    let Err(failure) = method.invoke(registry, Invocation::receiver_owned(7_u16)) else {
        panic!("an adapter without a local entry is unavailable")
    };
    assert!(matches!(
        failure.error().kind(),
        qubit_reflect::invoke::InvocationErrorKind::EntryUnavailable(
            qubit_reflect::invoke::InvocationDispatchReason::MissingEntry
        )
    ));
    let Some(qubit_reflect::invoke::InvocationReceiver::Owned(receiver)) = failure.recovery().receiver() else {
        panic!("the unavailable entry preserves its owned receiver")
    };
    assert_eq!(receiver.downcast_ref::<u16>(), Some(&7));
}

#[test]
fn test_method_instance_returns_borrowed_str_with_its_parameter_origin() {
    static ADAPTER: DescriptorInvocationAdapter = DescriptorInvocationAdapter::local(str_return_entry);
    let method = str_return_method(&ADAPTER);
    let registry = ReflectRegistry::initialize().expect("the registry must initialize");
    let input = "borrowed text";
    let output = method
        .invoke(
            registry,
            Invocation::associated([InvocationArg::Ref(DynamicRef::<Local>::new_str(input))]),
        )
        .expect("the str borrow remains tied to its caller input");
    let InvocationOutput::Ref { value, origins } = output else {
        panic!("the fixture returns a borrowed str")
    };
    assert_eq!(value.as_str(), Some(input));
    assert_eq!(origins.as_ref(), [BorrowOrigin::Parameter(0)]);
}
