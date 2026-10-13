// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Characterization tests for method descriptor structure and invocation.

use std::any::type_name;
use std::sync::LazyLock;

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

static EMPTY_GENERIC_DEFINITION: LazyLock<GenericDefinitionDescriptor> =
    LazyLock::new(|| GenericDefinitionDescriptor::new(Vec::new(), Vec::new()));

/// Creates the stable identity used by method descriptor fixtures.
fn method_identity(index: usize) -> MemberId {
    MemberId::new(
        "fixture::MethodTarget",
        "method",
        index,
        FragmentIdentity::new("fixture", "method_descriptor_tests", 1, 1, "method", index as u64),
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
