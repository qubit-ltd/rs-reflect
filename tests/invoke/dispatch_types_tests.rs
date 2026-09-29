// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Public type exports, exhaustive variants and positive thread boundaries.

use qubit_reflect::invoke::Invocation;
use qubit_reflect::invoke::InvocationDispatchMode;
use qubit_reflect::invoke::InvocationDispatchReason;
use qubit_reflect::invoke::InvocationDispatchResult;
use qubit_reflect::invoke::InvocationUnavailable;
use qubit_reflect::value::ThreadSafe;

#[test]
fn test_dispatch_modes_are_exhaustive() {
    let modes = [
        InvocationDispatchMode::Local,
        InvocationDispatchMode::ThreadSafe,
        InvocationDispatchMode::CatchingLocal,
        InvocationDispatchMode::CatchingThreadSafe,
        InvocationDispatchMode::PinnedRefLocal,
        InvocationDispatchMode::PinnedMutLocal,
    ];
    for mode in modes {
        let label = match mode {
            InvocationDispatchMode::Local => "Local",
            InvocationDispatchMode::ThreadSafe => "ThreadSafe",
            InvocationDispatchMode::CatchingLocal => "CatchingLocal",
            InvocationDispatchMode::CatchingThreadSafe => "CatchingThreadSafe",
            InvocationDispatchMode::PinnedRefLocal => "PinnedRefLocal",
            InvocationDispatchMode::PinnedMutLocal => "PinnedMutLocal",
        };
        assert_eq!(format!("{mode:?}"), label);
    }
}

#[test]
fn test_dispatch_reasons_are_exhaustive() {
    let reasons = [
        InvocationDispatchReason::NoAdapter { reasons: Box::new([]) },
        InvocationDispatchReason::MissingEntry,
        InvocationDispatchReason::CatchingNotRequested,
        InvocationDispatchReason::PanicAbort,
        InvocationDispatchReason::PinnedReceiverTypeMismatch,
    ];
    for reason in reasons {
        match reason {
            InvocationDispatchReason::NoAdapter { reasons } => assert!(reasons.is_empty()),
            InvocationDispatchReason::MissingEntry
            | InvocationDispatchReason::CatchingNotRequested
            | InvocationDispatchReason::PanicAbort
            | InvocationDispatchReason::PinnedReceiverTypeMismatch => {}
        }
    }
}

#[test]
fn test_thread_safe_unavailable_preserves_send() {
    fn assert_send<T: Send>() {}
    assert_send::<InvocationUnavailable<Invocation<'static, ThreadSafe>>>();
    assert_send::<InvocationDispatchResult<Invocation<'static, ThreadSafe>, ()>>();
}
