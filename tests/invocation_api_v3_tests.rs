// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::marker::PhantomPinned;
use std::sync::LazyLock;

use qubit_reflect::ReflectRegistry;
use qubit_reflect::descriptor::ImplDefinitionDescriptor;
use qubit_reflect::descriptor::ImplKind;
use qubit_reflect::descriptor::InvocationAdapter;
use qubit_reflect::descriptor::InvocationAdapter as DescriptorInvocationAdapter;
use qubit_reflect::descriptor::InvocationUnavailableReason;
use qubit_reflect::descriptor::MethodDeclarationOwner;
use qubit_reflect::descriptor::MethodDescriptor;
use qubit_reflect::descriptor::MethodImplementationSource;
use qubit_reflect::descriptor::MethodInstanceDescriptor;
use qubit_reflect::descriptor::MethodVisibility;
use qubit_reflect::expression::GenericDefinitionDescriptor;
use qubit_reflect::expression::TypeExpression;
use qubit_reflect::identity::FragmentIdentity;
use qubit_reflect::identity::MemberId;
use qubit_reflect::identity::Visibility;
use qubit_reflect::invoke::Invocation;
use qubit_reflect::invoke::InvocationErrorKind;
use qubit_reflect::invoke::InvocationFailure;
use qubit_reflect::invoke::InvocationOutput;
use qubit_reflect::invoke::PinnedRefAdapter;
use qubit_reflect::invoke::PinnedRefInvocation;
use qubit_reflect::invoke::PinnedRefInvocationFailure;
use qubit_reflect::value::Local;

static GENERICS: LazyLock<GenericDefinitionDescriptor> = LazyLock::new(|| GenericDefinitionDescriptor::new([], []));
static IMPL: LazyLock<ImplDefinitionDescriptor> = LazyLock::new(|| {
    ImplDefinitionDescriptor::new(
        FragmentIdentity::new("fixture", "invocation_api_v3_tests", 1, 1, "impl", 1),
        TypeExpression::Never,
        ImplKind::Inherent,
        None,
        &GENERICS,
    )
    .expect("the inherent fixture is valid")
});

fn identity(index: usize) -> MemberId {
    MemberId::new(
        "fixture::Target",
        "method",
        index,
        FragmentIdentity::new("fixture", "invocation_api_v3_tests", 1, 1, "method", index as u64),
    )
}

fn unit_entry<'call>(
    _registry: &ReflectRegistry,
    _invocation: Invocation<'call, Local>,
) -> Result<InvocationOutput<'call, Local>, InvocationFailure<'call, Local>> {
    Ok(InvocationOutput::Unit)
}

struct PinnedFixture {
    _pin: PhantomPinned,
}

fn pinned_ref_entry<'call>(
    _registry: &ReflectRegistry,
    invocation: PinnedRefInvocation<'call, PinnedFixture, Local>,
) -> Result<InvocationOutput<'call, Local>, PinnedRefInvocationFailure<'call, PinnedFixture, Local>> {
    let _pinned_receiver = invocation.receiver();
    Ok(InvocationOutput::Unit)
}

fn instance(
    index: usize,
    visibility: MethodVisibility,
    adapter: Option<&'static InvocationAdapter>,
    unavailable: Box<[InvocationUnavailableReason]>,
) -> MethodInstanceDescriptor {
    let method = Box::leak(Box::new(
        MethodDescriptor::builder(identity(index), "run", "run", MethodDeclarationOwner::Impl(&IMPL))
            .visibility(visibility)
            .build(),
    ));
    MethodInstanceDescriptor::new(method, None, MethodImplementationSource::Declared, adapter, unavailable)
        .expect("the fixture method instance is valid")
}

#[test]
fn test_invocation_builder_accepts_values_and_borrows() {
    let shared = String::from("shared");
    let mut mutable = 3_u16;
    let invocation = Invocation::receiver_mut(&mut mutable)
        .arg(7_u8)
        .arg_ref(&shared)
        .named_arg("label", String::from("ready"));
    assert_eq!(invocation.arguments().len(), 3);
    assert_eq!(invocation.argument_name(0), None);
    assert_eq!(invocation.argument_name(2), Some("label"));
}

#[test]
fn test_invocation_pin_builders_keep_the_pin_type() {
    let value = Box::pin(String::from("pinned"));
    let invocation = Invocation::receiver_pin_ref(value.as_ref()).arg(9_u8);
    assert_eq!(invocation.receiver().get_ref(), "pinned");
    assert_eq!(invocation.arguments().len(), 1);

    let mut value = Box::pin(4_u16);
    let invocation = Invocation::receiver_pin_mut(value.as_mut()).arg(10_u8);
    assert_eq!(invocation.arguments().len(), 1);
}

#[test]
fn test_invoke_rejects_private_method_and_unsafe_invoke_runs_adapter() {
    static ADAPTER: InvocationAdapter = InvocationAdapter::local(unit_entry);
    let method = instance(
        1,
        MethodVisibility::Declared(Visibility::Private),
        Some(&ADAPTER),
        Box::new([]),
    );
    let registry = ReflectRegistry::initialize().expect("the empty registry initializes");
    let Err(failure) = method.invoke(registry, Invocation::associated_function()) else {
        panic!("private methods are unavailable through the safe API")
    };
    assert_eq!(failure.error().kind(), &InvocationErrorKind::VisibilityDenied);
    assert!(failure.recovery().receiver().is_none());

    let output = unsafe { method.invoke_unsafe(registry, Invocation::associated_function()) }
        .expect("the unsafe API bypasses visibility and invokes the adapter");
    assert!(matches!(output, InvocationOutput::Unit));
}

#[test]
fn test_unbound_generic_method_is_reported_in_single_failure() {
    let method = instance(
        2,
        MethodVisibility::Declared(Visibility::Public),
        None,
        Box::new([InvocationUnavailableReason::UnspecializedGeneric]),
    );
    let registry = ReflectRegistry::initialize().expect("the empty registry initializes");
    let Err(failure) = method.invoke(registry, Invocation::associated_function()) else {
        panic!("an unbound generic method has no invocation adapter")
    };
    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::MethodUnavailable { reasons }
            if reasons.as_ref() == [InvocationUnavailableReason::UnspecializedGeneric]
    ));
}

#[test]
fn test_public_visibility_metadata_is_retained() {
    let method = instance(
        3,
        MethodVisibility::Declared(Visibility::Public),
        None,
        Box::new([InvocationUnavailableReason::UnspecializedGeneric]),
    );
    assert_eq!(
        method.effective_method().visibility(),
        &MethodVisibility::Declared(Visibility::Public)
    );
}

#[test]
fn test_unsafe_pinned_invoke_keeps_pin_and_bypasses_only_visibility() {
    static PINNED_ENTRY: PinnedRefAdapter<PinnedFixture, Local> = pinned_ref_entry;
    static ADAPTER: DescriptorInvocationAdapter = DescriptorInvocationAdapter::pinned_ref_local(&PINNED_ENTRY);
    let method = instance(
        4,
        MethodVisibility::Declared(Visibility::Private),
        Some(&ADAPTER),
        Box::new([]),
    );
    let registry = ReflectRegistry::initialize().expect("the empty registry initializes");
    let value = Box::pin(PinnedFixture { _pin: PhantomPinned });
    let invocation = Invocation::receiver_pin_ref(value.as_ref());
    let Err(failure) = method.invoke_pinned_ref(registry, invocation) else {
        panic!("the safe entry rejects a private method")
    };
    assert_eq!(failure.error().kind(), &InvocationErrorKind::VisibilityDenied);
    assert!(std::ptr::eq(
        failure.recovery().receiver().get_ref(),
        value.as_ref().get_ref()
    ));

    let output = unsafe { method.invoke_pinned_ref_unsafe(registry, Invocation::receiver_pin_ref(value.as_ref())) }
        .expect("the unsafe entry bypasses visibility while preserving Pin");
    assert!(matches!(output, InvocationOutput::Unit));
}
