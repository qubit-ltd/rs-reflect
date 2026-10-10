// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Tests unavailable input ownership and unconstrained diagnostics.

use std::error::Error;

use crate::descriptor::InvocationUnavailableReason;
use crate::invoke::InvocationDispatchMode;
use crate::invoke::InvocationDispatchReason;
use crate::invoke::InvocationUnavailable;

struct NonDebugInput<'input> {
    value: &'input str,
}

#[test]
fn test_unavailable_diagnostics_without_input_debug_or_static() {
    let input = String::from("borrowed secret input");
    let unavailable = InvocationUnavailable::new(
        InvocationDispatchMode::Local,
        InvocationDispatchReason::MissingEntry,
        NonDebugInput { value: &input },
    );
    assert_eq!(unavailable.mode(), InvocationDispatchMode::Local);
    assert_eq!(unavailable.reason(), &InvocationDispatchReason::MissingEntry);
    assert_eq!(unavailable.invocation().value, input);
    let diagnostic = format!("{unavailable:?}");
    assert!(diagnostic.contains("Local"));
    assert!(diagnostic.contains("MissingEntry"));
    assert!(!diagnostic.contains(&input));
    assert!(unavailable.to_string().contains("missing"));
    let error: &dyn Error = &unavailable;
    assert!(error.source().is_none());
    let (mode, reason, recovered) = unavailable.into_parts();
    assert_eq!(mode, InvocationDispatchMode::Local);
    assert_eq!(reason, InvocationDispatchReason::MissingEntry);
    assert!(std::ptr::eq(recovered.value, input.as_str()));
}

#[test]
fn test_into_invocation_preserves_original_allocation() {
    let input = Box::new(47_u8);
    let original = std::ptr::from_ref(input.as_ref());
    let unavailable = InvocationUnavailable::new(
        InvocationDispatchMode::ThreadSafe,
        InvocationDispatchReason::PanicAbort,
        input,
    );
    let recovered = unavailable.into_invocation();
    assert_eq!(std::ptr::from_ref(recovered.as_ref()), original);
}

#[test]
fn test_into_parts_preserves_ordered_no_adapter_reasons() {
    let reasons = Box::new([
        InvocationUnavailableReason::UnsafeMethod,
        InvocationUnavailableReason::DisabledByPolicy,
        InvocationUnavailableReason::UnsafeMethod,
    ]);
    let original = reasons.as_ptr();
    let unavailable = InvocationUnavailable::new(
        InvocationDispatchMode::CatchingLocal,
        InvocationDispatchReason::NoAdapter { reasons },
        47_u8,
    );
    let (mode, reason, invocation) = unavailable.into_parts();
    assert_eq!(mode, InvocationDispatchMode::CatchingLocal);
    assert_eq!(invocation, 47);
    let InvocationDispatchReason::NoAdapter { reasons } = reason else {
        panic!("dispatch must preserve its original structured reason");
    };
    assert_eq!(reasons.as_ptr(), original);
    assert_eq!(
        reasons.as_ref(),
        [
            InvocationUnavailableReason::UnsafeMethod,
            InvocationUnavailableReason::DisabledByPolicy,
            InvocationUnavailableReason::UnsafeMethod,
        ]
    );
}
