// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Integration tests for typed pinned invocation inputs and recovery.

use std::error::Error;
use std::pin::Pin;

use qubit_reflect::identity::FragmentIdentity;
use qubit_reflect::identity::MemberId;
use qubit_reflect::invoke::ArgumentExpectation;
use qubit_reflect::invoke::InvocationArg;
use qubit_reflect::invoke::InvocationBinding;
use qubit_reflect::invoke::PinnedMutInvocation;
use qubit_reflect::invoke::PinnedRefInvocation;
use qubit_reflect::value::DynamicOwned;
use qubit_reflect::value::Local;

/// Creates the method identity used by pinned validation fixtures.
fn method_identity() -> MemberId {
    MemberId::new(
        "example::Pinned",
        "method",
        0,
        FragmentIdentity::new("example", "example::Pinned", 1, 1, "method", 0),
    )
}

/// Verifies shared pinned invocation success and failure preserve the exact pin
/// and caller binding metadata.
#[test]
fn test_pinned_ref_invocation_preserves_pin_arguments_and_recovery() {
    let receiver = 7_u8;
    let invocation = PinnedRefInvocation::<u8, Local>::new(
        Pin::new(&receiver),
        [InvocationArg::Owned(DynamicOwned::<Local>::new(11_u16))],
    );
    assert_eq!(*invocation.receiver(), 7);
    assert_eq!(invocation.arguments().len(), 1);
    assert_eq!(invocation.argument_name(0), None);
    let validated = invocation
        .validate(&method_identity(), &[ArgumentExpectation::owned::<u16>()])
        .unwrap_or_else(|_| panic!("matching pinned shared invocation should validate"));
    let (validated_receiver, arguments) = validated.into_parts();
    assert_eq!(*validated_receiver, 7);
    assert_eq!(arguments.len(), 1);

    let invocation = PinnedRefInvocation::<u8, Local>::from_bindings(
        Pin::new(&receiver),
        [InvocationBinding::named(
            "value",
            InvocationArg::Owned(DynamicOwned::<Local>::new(13_u16)),
        )],
    );
    assert_eq!(invocation.argument_name(0), Some("value"));
    let Err(failure) = invocation.validate(&method_identity(), &[ArgumentExpectation::owned::<String>()]) else {
        panic!("mismatched pinned shared argument should fail");
    };
    assert!(format!("{failure:?}").contains("PinnedRefInvocationFailure"));
    assert!(failure.to_string().contains("failed to invoke"));
    assert!(failure.source().is_some());
    assert_eq!(*failure.recovery.receiver(), 7);
    assert_eq!(failure.recovery.arguments().len(), 1);
    assert_eq!(failure.recovery.argument_name(0), Some("value"));
    let recovered = failure.recovery.into_invocation();
    assert_eq!(recovered.argument_name(0), Some("value"));
}

/// Verifies mutable pinned invocation success and failure retain a reborrowable
/// pin and can reconstruct the original typed invocation.
#[test]
fn test_pinned_mut_invocation_preserves_pin_arguments_and_recovery() {
    let mut receiver = 17_u8;
    let invocation = PinnedMutInvocation::<u8, Local>::new(
        Pin::new(&mut receiver),
        [InvocationArg::Owned(DynamicOwned::<Local>::new(19_u16))],
    );
    assert_eq!(invocation.arguments().len(), 1);
    assert_eq!(invocation.argument_name(0), None);
    {
        let validated = invocation
            .validate(&method_identity(), &[ArgumentExpectation::owned::<u16>()])
            .unwrap_or_else(|_| panic!("matching pinned mutable invocation should validate"));
        let (mut validated_receiver, arguments) = validated.into_parts();
        *validated_receiver.as_mut().get_mut() = 23;
        assert_eq!(arguments.len(), 1);
    }
    assert_eq!(receiver, 23);

    let invocation = PinnedMutInvocation::<u8, Local>::from_bindings(
        Pin::new(&mut receiver),
        [InvocationBinding::named(
            "value",
            InvocationArg::Owned(DynamicOwned::<Local>::new(29_u16)),
        )],
    );
    assert_eq!(invocation.argument_name(0), Some("value"));
    let Err(mut failure) = invocation.validate(&method_identity(), &[ArgumentExpectation::owned::<String>()]) else {
        panic!("mismatched pinned mutable argument should fail");
    };
    assert!(format!("{failure:?}").contains("PinnedMutInvocationFailure"));
    assert!(failure.to_string().contains("failed to invoke"));
    assert!(failure.source().is_some());
    assert_eq!(*failure.recovery.receiver(), 23);
    assert_eq!(failure.recovery.arguments().len(), 1);
    assert_eq!(failure.recovery.argument_name(0), Some("value"));
    let recovered = failure.recovery.into_invocation();
    assert_eq!(recovered.argument_name(0), Some("value"));
}
