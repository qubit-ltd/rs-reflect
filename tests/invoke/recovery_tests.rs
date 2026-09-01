// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Integration tests for ordinary invocation recovery inspection APIs.

use std::error::Error;

use qubit_reflect::identity::FragmentIdentity;
use qubit_reflect::identity::MemberId;
use qubit_reflect::invoke::ArgumentExpectation;
use qubit_reflect::invoke::Invocation;
use qubit_reflect::invoke::InvocationArg;
use qubit_reflect::invoke::InvocationBinding;
use qubit_reflect::invoke::ReceiverExpectation;
use qubit_reflect::value::DynamicOwned;
use qubit_reflect::value::Local;
use qubit_reflect::value::ThreadSafe;

/// Creates the method identity used by recovery validation fixtures.
fn method_identity() -> MemberId {
    MemberId::new(
        "example::Recovery",
        "method",
        1,
        FragmentIdentity::new(
            "example",
            "example::Recovery",
            1,
            1,
            "method",
            1,
        ),
    )
}

/// Verifies a failed associated invocation exposes, formats, and reconstructs
/// all caller-ordered bindings.
#[test]
fn test_invocation_recovery_inspection_preserves_bindings() {
    let invocation =
        Invocation::associated_bindings([InvocationBinding::named(
            "value",
            InvocationArg::Owned(DynamicOwned::<Local>::new(7_u8)),
        )]);
    let failure = invocation
        .validate(
            &method_identity(),
            ReceiverExpectation::none(),
            &[ArgumentExpectation::owned::<String>()],
        )
        .expect_err("mismatched argument type should fail validation");

    assert!(failure.recovery.receiver().is_none());
    assert_eq!(failure.recovery.arguments().len(), 1);
    assert_eq!(failure.recovery.argument_name(0), Some("value"));
    assert_eq!(failure.recovery.argument_name(1), None);
    assert!(format!("{:?}", failure.recovery).contains("InvocationRecovery"));
    assert!(format!("{failure:?}").contains("InvocationFailure"));
    assert!(failure.to_string().contains("failed to invoke"));
    assert!(failure.source().is_some());

    let recovered = failure.recovery.into_invocation();
    assert_eq!(recovered.argument_name(0), Some("value"));
}

/// Verifies successful validation exposes and consumes receiver and argument
/// state in both erased-value modes.
#[test]
fn test_validated_invocation_inspection_supports_both_modes() {
    let mutable_expectation = ArgumentExpectation::borrowed_mut::<u32>();
    assert_eq!(
        mutable_expectation.type_name(),
        std::any::type_name::<u32>()
    );

    let local = Invocation::<Local>::associated([])
        .validate(&method_identity(), ReceiverExpectation::none(), &[])
        .expect("an empty associated local invocation must validate");
    assert!(local.receiver().is_none());
    assert!(local.arguments().is_empty());
    assert!(format!("{local:?}").contains("ValidatedInvocation"));
    let (receiver, arguments) = local.into_parts();
    assert!(receiver.is_none());
    assert!(arguments.is_empty());

    let thread_safe = Invocation::<ThreadSafe>::associated([])
        .validate(&method_identity(), ReceiverExpectation::none(), &[])
        .expect("an empty associated thread-safe invocation must validate");
    assert!(thread_safe.receiver().is_none());
    assert!(thread_safe.arguments().is_empty());
    assert!(format!("{thread_safe:?}").contains("ValidatedInvocation"));
    let (receiver, arguments) = thread_safe.into_parts();
    assert!(receiver.is_none());
    assert!(arguments.is_empty());
}
