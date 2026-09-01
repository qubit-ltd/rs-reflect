// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Integration tests for invocation diagnostics and caught-panic payloads.

use std::any::TypeId;
use std::error::Error;

use qubit_reflect::identity::FragmentIdentity;
use qubit_reflect::identity::MemberId;
use qubit_reflect::invoke::InvocationError;
use qubit_reflect::invoke::InvocationErrorKind;
use qubit_reflect::invoke::InvocationInputMode;
use qubit_reflect::invoke::InvocationPanic;

/// Creates a stable method identity used across diagnostic fixtures.
fn method_identity() -> MemberId {
    MemberId::new(
        "example::Service",
        "method",
        2,
        FragmentIdentity::new("example", "example::Service", 1, 1, "method", 2),
    )
}

/// Verifies each validation category produces contextual diagnostic text and
/// remains available as the source of the enclosing error.
#[test]
fn test_invocation_error_kinds_format_complete_context() {
    let kinds = [
        InvocationErrorKind::ReceiverModeMismatch {
            expected: Some(InvocationInputMode::Ref),
            actual: Some(InvocationInputMode::Mut),
        },
        InvocationErrorKind::ReceiverTypeMismatch {
            expected: TypeId::of::<u8>(),
            actual: TypeId::of::<u16>(),
            expected_name: "u8",
        },
        InvocationErrorKind::ReceiverAdapterUnavailable {
            expected_name: "Receiver",
        },
        InvocationErrorKind::ReceiverAdapterRejected {
            expected_name: "Receiver",
        },
        InvocationErrorKind::ArgumentCountMismatch { expected: 2, actual: 1 },
        InvocationErrorKind::UnknownArgumentName {
            input_index: 0,
            name: "unknown".into(),
        },
        InvocationErrorKind::AmbiguousArgumentName {
            input_index: 0,
            name: "value".into(),
            parameter_indices: vec![0, 1].into_boxed_slice(),
        },
        InvocationErrorKind::NamedArgumentUnavailable {
            input_index: 0,
            parameter_index: 1,
            name: "value".into(),
        },
        InvocationErrorKind::DuplicateArgumentBinding {
            input_index: 1,
            parameter_index: 0,
        },
        InvocationErrorKind::PositionalArgumentOverflow { input_index: 2 },
        InvocationErrorKind::MissingArgumentBinding {
            parameter_index: 1,
            name: Some("value"),
        },
        InvocationErrorKind::NamedBindingRequiresDescriptor {
            input_index: 0,
            name: "value".into(),
        },
        InvocationErrorKind::ArgumentModeMismatch {
            index: 0,
            expected: InvocationInputMode::Owned,
            actual: InvocationInputMode::Ref,
        },
        InvocationErrorKind::ArgumentTypeMismatch {
            index: 0,
            expected: TypeId::of::<u8>(),
            actual: TypeId::of::<u16>(),
            expected_name: "u8",
        },
    ];

    for kind in kinds {
        assert!(!kind.to_string().is_empty());
        let error = InvocationError::new(method_identity(), kind.clone());
        assert_eq!(error.method_identity(), &method_identity());
        assert_eq!(error.kind(), &kind);
        assert!(error.to_string().contains("failed to invoke method at index 2"));
        assert!(error.source().is_some());
    }
}

/// Verifies caught panic payload downcasts either return the exact payload or
/// preserve the original error intact.
#[test]
fn test_invocation_panic_preserves_payload_and_identity() {
    let panic = InvocationPanic::new(method_identity(), Box::new(String::from("boom")));
    assert_eq!(panic.method_identity(), &method_identity());
    assert!(panic.payload().is::<String>());
    assert!(format!("{panic:?}").contains("InvocationPanic"));
    assert!(panic.to_string().contains("reflected method at index 2"));

    let panic = panic
        .downcast_payload::<u8>()
        .expect_err("wrong payload type should preserve the panic");
    assert_eq!(
        panic
            .downcast_payload::<String>()
            .unwrap_or_else(|_| panic!("exact payload type should downcast")),
        "boom",
    );
}
