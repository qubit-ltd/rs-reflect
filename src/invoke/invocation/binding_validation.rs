// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Compatibility checks for invocation input modes.

use crate::invoke::InvocationInputMode;

/// Checks whether an argument supplied in `actual` mode can satisfy `expected`.
///
/// A mutable reference can satisfy a shared-reference expectation; all other
/// modes must match exactly.
#[must_use]
#[inline]
pub(in crate::invoke::invocation) fn mode_matches(expected: InvocationInputMode, actual: InvocationInputMode) -> bool {
    expected == actual || (expected == InvocationInputMode::Ref && actual == InvocationInputMode::Mut)
}

/// Checks whether optional receiver modes are compatible.
///
/// Two absent modes are compatible, and two present modes use the same
/// compatibility rule as [`mode_matches`]. A present mode cannot satisfy an
/// absent mode, or vice versa.
#[must_use]
#[inline]
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
