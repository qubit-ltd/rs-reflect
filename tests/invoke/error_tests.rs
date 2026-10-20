// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::Invocation;
use qubit_reflect::invoke::CaughtInvocationFailure;
use qubit_reflect::value::Local;

use super::fixtures::Worker;
use super::fixtures::method;

#[test]
fn test_catching_invocation_reports_user_panic_as_a_separate_failure() {
    let worker = Worker::default();
    let Err(CaughtInvocationFailure::Panicked(panic)) =
        method("panic").invoke_catching(Invocation::<Local>::on_ref(&worker))
    else {
        panic!("a panic after method entry must be reported as Panicked");
    };

    assert_eq!(panic.payload().downcast_ref::<&str>(), Some(&"worker panic payload"));
}
