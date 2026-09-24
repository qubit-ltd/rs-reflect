// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow explicit-imports
//! Characterization tests for method descriptor structure and invocation.

use std::any::TypeId;
use std::any::type_name;
use std::sync::LazyLock;

use qubit_reflect::ReflectRegistry;
use qubit_reflect::descriptor::CatchingAvailability;
use qubit_reflect::descriptor::ImplDefinitionDescriptor;
use qubit_reflect::descriptor::ImplKind;
use qubit_reflect::descriptor::InvocationAdapter;
use qubit_reflect::descriptor::InvocationUnavailableReason;
use qubit_reflect::descriptor::MethodDeclarationOwner;
use qubit_reflect::descriptor::MethodDescriptor;
use qubit_reflect::descriptor::MethodDescriptorBuilder;
use qubit_reflect::descriptor::MethodImplementationSource;
use qubit_reflect::descriptor::MethodInstanceBuildError;
use qubit_reflect::descriptor::MethodInstanceDescriptor;
use qubit_reflect::descriptor::MethodQualifiers;
use qubit_reflect::descriptor::MethodVisibility;
use qubit_reflect::descriptor::ParameterDescriptor;
use qubit_reflect::descriptor::ParameterPassingMode;
use qubit_reflect::descriptor::ParameterPatternDescriptor;
use qubit_reflect::descriptor::ReceiverDescriptor;
use qubit_reflect::descriptor::ReturnDescriptor;
use qubit_reflect::descriptor::ReturnKind;
use qubit_reflect::expression::GenericDefinitionDescriptor;
use qubit_reflect::expression::TypeExpression;
use qubit_reflect::identity::FragmentIdentity;
use qubit_reflect::identity::MemberId;
use qubit_reflect::identity::Visibility;
use qubit_reflect::invoke::ArgumentExpectation;
use qubit_reflect::invoke::CatchingInvocationResult;
use qubit_reflect::invoke::Invocation;
use qubit_reflect::invoke::InvocationArg;
use qubit_reflect::invoke::InvocationBinding;
use qubit_reflect::invoke::InvocationFailure;
use qubit_reflect::invoke::InvocationOutput;
use qubit_reflect::invoke::ReceiverExpectation;
use qubit_reflect::value::DynamicOwned;
use qubit_reflect::value::Local;
use qubit_reflect::value::ThreadSafe;

static EMPTY_GENERIC_DEFINITION: LazyLock<GenericDefinitionDescriptor> =
    LazyLock::new(|| GenericDefinitionDescriptor::new(Vec::new(), Vec::new()));

/// Creates the stable identity used by method descriptor fixtures.
fn method_identity(index: usize) -> MemberId {
    MemberId::new(
        "fixture::MethodTarget",
        "method",
        index,
        FragmentIdentity::new(
            "fixture",
            "method_descriptor_tests",
            1,
            1,
            "method",
            index as u64,
        ),
    )
}

/// Returns an inherent impl owner for hand-written method declarations.
fn impl_definition() -> &'static ImplDefinitionDescriptor {
    static DEFINITION: LazyLock<ImplDefinitionDescriptor> = LazyLock::new(|| {
        ImplDefinitionDescriptor::new(
            FragmentIdentity::new("fixture", "method_descriptor_tests", 1, 1, "impl", 1),
            TypeExpression::Never,
            ImplKind::Inherent,
            None,
            &EMPTY_GENERIC_DEFINITION,
        )
        .expect("the inherent fixture definition must be valid")
    });
    &DEFINITION
}

/// Constructs one owned parameter with a caller-visible name and index.
fn parameter(index: usize, name: &'static str) -> ParameterDescriptor {
    ParameterDescriptor::new(
        index,
        Some(name),
        ParameterPatternDescriptor::Identifier,
        ParameterPassingMode::Owned,
        TypeExpression::Never,
        None,
    )
}

/// Returns one successful local invocation result.
fn return_local<'call>(
    _registry: &ReflectRegistry,
    _invocation: Invocation<'call, Local>,
) -> Result<InvocationOutput<'call, Local>, InvocationFailure<'call, Local>> {
    Ok(InvocationOutput::Owned(DynamicOwned::<Local>::new(7_u8)))
}

/// Returns one successful thread-safe invocation result.
fn return_thread_safe<'call>(
    _registry: &ReflectRegistry,
    _invocation: Invocation<'call, ThreadSafe>,
) -> Result<InvocationOutput<'call, ThreadSafe>, InvocationFailure<'call, ThreadSafe>> {
    Ok(InvocationOutput::Owned(DynamicOwned::<ThreadSafe>::new(
        8_u8,
    )))
}

/// Wraps the local fixture output in the catching contract.
fn catch_local<'call>(
    registry: &ReflectRegistry,
    invocation: Invocation<'call, Local>,
) -> CatchingInvocationResult<'call, Local> {
    return_local(registry, invocation).map(Ok)
}

/// Wraps the thread-safe fixture output in the catching contract.
fn catch_thread_safe<'call>(
    registry: &ReflectRegistry,
    invocation: Invocation<'call, ThreadSafe>,
) -> CatchingInvocationResult<'call, ThreadSafe> {
    return_thread_safe(registry, invocation).map(Ok)
}

/// Validates the declaration-ordered inputs and deliberately rejects the
/// caller's final value type.
fn validate_local_types<'call>(
    _registry: &ReflectRegistry,
    invocation: Invocation<'call, Local>,
) -> Result<InvocationOutput<'call, Local>, InvocationFailure<'call, Local>> {
    invocation.validate(
        &method_identity(7),
        ReceiverExpectation::none(),
        &[
            ArgumentExpectation::owned::<u8>(),
            ArgumentExpectation::owned::<u16>(),
            ArgumentExpectation::owned::<u32>(),
        ],
    )?;
    unreachable!("the mismatched fixture input must fail validation")
}

/// Returns the fully qualified compiler name for a public descriptor type.
fn public_type_name<T>() -> &'static str {
    type_name::<T>()
}

#[test]
fn test_method_descriptor_types_remain_available_from_descriptor_module() {
    let public_types = [
        public_type_name::<CatchingAvailability>(),
        public_type_name::<InvocationAdapter>(),
        public_type_name::<InvocationUnavailableReason>(),
        public_type_name::<MethodDeclarationOwner>(),
        public_type_name::<MethodDescriptor>(),
        public_type_name::<MethodDescriptorBuilder>(),
        public_type_name::<MethodImplementationSource>(),
        public_type_name::<MethodInstanceBuildError>(),
        public_type_name::<MethodInstanceDescriptor>(),
        public_type_name::<MethodQualifiers>(),
        public_type_name::<MethodVisibility>(),
        public_type_name::<ParameterDescriptor>(),
        public_type_name::<ParameterPassingMode>(),
        public_type_name::<ParameterPatternDescriptor>(),
        public_type_name::<ReceiverDescriptor>(),
        public_type_name::<ReturnDescriptor>(),
        public_type_name::<ReturnKind>(),
    ];
    assert!(public_types.iter().all(|name| !name.is_empty()));
}

#[test]
fn test_method_descriptor_builder_preserves_parameter_and_debug_field_order() {
    let method = MethodDescriptor::builder(
        method_identity(3),
        "encode",
        "encode_values",
        MethodDeclarationOwner::Impl(impl_definition()),
    )
    .visibility(MethodVisibility::Declared(Visibility::Public))
    .receiver(Some(ReceiverDescriptor::Shared))
    .parameters(vec![
        parameter(2, "third"),
        parameter(0, "first"),
        parameter(1, "second"),
    ])
    .return_value(ReturnDescriptor::new(ReturnKind::Never, None, None))
    .qualifiers(MethodQualifiers::new(false, true, false, None, false))
    .generic_definition(&EMPTY_GENERIC_DEFINITION)
    .has_default(true)
    .build();

    assert_eq!(
        method
            .parameters()
            .iter()
            .map(ParameterDescriptor::name)
            .collect::<Vec<_>>(),
        [Some("third"), Some("first"), Some("second")]
    );
    let debug = format!("{method:?}");
    let mut cursor = 0;
    for field in [
        "identity:",
        "rust_name:",
        "query_name:",
        "visibility:",
        "receiver:",
        "parameters:",
        "return_value:",
        "qualifiers:",
        "generic_definition:",
        "has_default:",
        "declaration_owner:",
    ] {
        let offset = debug[cursor..]
            .find(field)
            .unwrap_or_else(|| panic!("Debug output must retain the `{field}` field"));
        cursor += offset + field.len();
    }
}

#[test]
fn test_invocation_adapter_mode_availability_remains_distinct() {
    let registry = ReflectRegistry::initialize().expect("the fixture registry must initialize");
    let local = InvocationAdapter::local_with_catching(return_local, catch_local);
    assert!(
        local
            .invoke_local(registry, Invocation::associated([]))
            .is_some()
    );
    assert!(
        local
            .invoke_thread_safe(registry, Invocation::associated([]))
            .is_none()
    );
    assert!(
        local
            .invoke_catching_local(registry, Invocation::associated([]))
            .is_some()
    );
    assert!(
        local
            .invoke_catching_thread_safe(registry, Invocation::associated([]))
            .is_none()
    );
    assert_eq!(
        local.catching_availability(),
        CatchingAvailability::Available
    );

    let thread_safe =
        InvocationAdapter::thread_safe_with_catching(return_thread_safe, catch_thread_safe);
    assert!(
        thread_safe
            .invoke_local(registry, Invocation::associated([]))
            .is_none()
    );
    assert!(
        thread_safe
            .invoke_thread_safe(registry, Invocation::associated([]))
            .is_some()
    );
    assert!(
        thread_safe
            .invoke_catching_local(registry, Invocation::associated([]))
            .is_none()
    );
    assert!(
        thread_safe
            .invoke_catching_thread_safe(registry, Invocation::associated([]))
            .is_some()
    );
    assert_eq!(
        thread_safe.catching_availability(),
        CatchingAvailability::Available
    );
}

#[test]
fn test_method_instance_build_error_messages_remain_stable() {
    let cases = [
        (
            MethodInstanceBuildError::DeclaredMethodNotOwnedByImpl,
            "a declared inherent method must be owned by an impl",
        ),
        (
            MethodInstanceBuildError::TraitMethodNotOwnedByTrait,
            "a trait method instance must be owned by a trait",
        ),
        (
            MethodInstanceBuildError::RequiredMethodHasAdapter,
            "a required method cannot have an invocation adapter",
        ),
        (
            MethodInstanceBuildError::OverriddenMethodMissingImplementation,
            "an overridden method must name its impl method",
        ),
        (
            MethodInstanceBuildError::UnexpectedImplementationMethod,
            "only an overridden method can name an impl method",
        ),
        (
            MethodInstanceBuildError::AdapterHasUnavailableReasons,
            "an available invocation adapter cannot have unavailable reasons",
        ),
        (
            MethodInstanceBuildError::UnavailableMethodMissingReasons,
            "an unavailable method must provide a structured reason",
        ),
    ];
    for (error, expected) in cases {
        assert_eq!(error.to_string(), expected);
    }
}

#[test]
fn test_method_instance_validation_failure_recovers_caller_order() {
    static ADAPTER: InvocationAdapter = InvocationAdapter::local(validate_local_types);
    let declaration = Box::leak(Box::new(
        MethodDescriptor::builder(
            method_identity(7),
            "encode",
            "encode",
            MethodDeclarationOwner::Impl(impl_definition()),
        )
        .parameters(vec![
            parameter(0, "first"),
            parameter(1, "second"),
            parameter(2, "third"),
        ])
        .build(),
    ));
    let instance = MethodInstanceDescriptor::new(
        declaration,
        None,
        MethodImplementationSource::Declared,
        Some(&ADAPTER),
        Box::new([]),
    )
    .expect("the fixture method instance must be valid");
    let invocation = Invocation::associated_bindings([
        InvocationBinding::named(
            "third",
            InvocationArg::Owned(DynamicOwned::<Local>::new(12_u32)),
        ),
        InvocationBinding::positional(InvocationArg::Owned(DynamicOwned::<Local>::new(10_u8))),
        InvocationBinding::named(
            "second",
            InvocationArg::Owned(DynamicOwned::<Local>::new(String::from("wrong"))),
        ),
    ]);

    let result = instance
        .invoke_local(
            ReflectRegistry::initialize().expect("the fixture registry must initialize"),
            invocation,
        )
        .expect("the local adapter must be available");
    let Err(failure) = result else {
        panic!("the mismatched second parameter must fail validation")
    };
    assert_eq!(
        (0..3)
            .map(|index| failure.recovery().argument_name(index))
            .collect::<Vec<_>>(),
        [Some("third"), None, Some("second")]
    );
    assert_eq!(
        failure
            .recovery()
            .arguments()
            .iter()
            .map(InvocationArg::type_id)
            .collect::<Vec<TypeId>>(),
        [
            TypeId::of::<u32>(),
            TypeId::of::<u8>(),
            TypeId::of::<String>()
        ]
    );
}
