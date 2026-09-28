// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! AssociatedConstReadUnavailableReason metadata and behavior.

/// Why an associated constant has no safe owned-value reader.
///
/// This reason is reported when generated code cannot prove the value can cross
/// the owned dynamic boundary.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::AssociatedConstReadUnavailableReason;
/// let reason = AssociatedConstReadUnavailableReason::UnprovenOwnedValue;
/// assert_eq!(reason, AssociatedConstReadUnavailableReason::UnprovenOwnedValue);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AssociatedConstReadUnavailableReason {
    /// The generated code cannot prove that the declared value type is sized
    /// and `'static`, as required by the local owned dynamic boundary.
    UnprovenOwnedValue,
}
