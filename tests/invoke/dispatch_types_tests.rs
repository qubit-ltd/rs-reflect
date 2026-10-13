// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Remaining dispatch diagnostics and local invocation paths.

use qubit_reflect::invoke::InvocationDispatchMode;
use qubit_reflect::invoke::InvocationDispatchReason;

#[test]
fn test_local_dispatch_modes_have_stable_names() {
    let modes = [
        InvocationDispatchMode::Local,
        InvocationDispatchMode::CatchingLocal,
        InvocationDispatchMode::PinnedRefLocal,
        InvocationDispatchMode::PinnedMutLocal,
    ];
    for mode in modes {
        let label = match mode {
            InvocationDispatchMode::Local => "Local",
            InvocationDispatchMode::CatchingLocal => "CatchingLocal",
            InvocationDispatchMode::PinnedRefLocal => "PinnedRefLocal",
            InvocationDispatchMode::PinnedMutLocal => "PinnedMutLocal",
            _ => panic!("thread-safe dispatch modes are not part of the v3 contract"),
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
