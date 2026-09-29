// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Structured explanations for unavailable invocation entries.

use crate::descriptor::InvocationUnavailableReason;

/// Why dispatch rejected a request before validation or execution.
///
/// These reasons describe entry availability, independently of invocation
/// argument validation or a panic from an executed method.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::InvocationUnavailableReason;
/// use qubit_reflect::invoke::InvocationDispatchReason;
///
/// let reason = InvocationDispatchReason::NoAdapter {
///     reasons: Box::new([InvocationUnavailableReason::UnsafeMethod]),
/// };
/// assert!(matches!(reason, InvocationDispatchReason::NoAdapter { .. }));
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InvocationDispatchReason {
    /// The method instance has no invocation adapter.
    NoAdapter {
        /// Original method unavailability reasons, retaining their order.
        ///
        /// The static descriptor reasons are copied only on this failure path.
        reasons: Box<[InvocationUnavailableReason]>,
    },
    /// The adapter lacks the entry requested by the caller.
    MissingEntry,
    /// An ordinary entry exists, but panic capture was not requested in
    /// metadata.
    CatchingNotRequested,
    /// An ordinary entry exists, but the panic strategy prevents capture.
    PanicAbort,
    /// A pinned entry exists, but its concrete receiver type differs from the
    /// input.
    PinnedReceiverTypeMismatch,
}
