// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Compatibility checks for invocation input modes.

use crate::invoke::InvocationInputMode;

/// Returns whether `actual` can safely satisfy `expected`.
pub(in crate::invoke::invocation) fn mode_matches(expected: InvocationInputMode, actual: InvocationInputMode) -> bool {
    expected == actual || (expected == InvocationInputMode::Ref && actual == InvocationInputMode::Mut)
}

/// Applies argument mode compatibility to optional receiver modes.
pub(in crate::invoke::invocation) fn receiver_mode_matches(
    expected: Option<InvocationInputMode>,
    actual: Option<InvocationInputMode>,
) -> bool {
    match (expected, actual) {
        (None, None) => true,
        (Some(expected), Some(actual)) => mode_matches(expected, actual),
        (None, Some(_)) | (Some(_), None) => false,
    }
}
