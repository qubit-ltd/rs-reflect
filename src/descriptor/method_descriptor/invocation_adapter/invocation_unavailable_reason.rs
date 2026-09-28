// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Reasons that explain unavailable invocation modes.

/// A stable reason why a concrete method instance cannot be invoked
/// dynamically.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::InvocationUnavailableReason;
/// let reason = InvocationUnavailableReason::UnsafeMethod;
/// assert_eq!(reason, InvocationUnavailableReason::UnsafeMethod);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum InvocationUnavailableReason {
    /// The receiver form has no safe adapter.
    UnsupportedReceiver,
    /// A parameter pattern has no safe adapter.
    UnsupportedParameterPattern,
    /// The declaration is generic and has no registered specialization.
    UnspecializedGeneric,
    /// A concrete specialization was registered but has no safe generated
    /// invocation adapter.
    UnsupportedSpecialization,
    /// The declaration is unsafe.
    UnsafeMethod,
    /// The declared ABI has no safe adapter.
    UnsupportedAbi,
    /// Variadic invocation is not supported.
    Variadic,
    /// The return borrow cannot be related safely to an input borrow.
    UnsupportedBorrowedReturn,
    /// An opaque return value cannot cross the dynamic boundary.
    OpaqueReturn,
    /// An unsized value has no dedicated safe adapter.
    UnsupportedUnsizedValue,
    /// A default method has call-site bounds that reflection cannot prove.
    UnprovenDefaultConstraint,
    /// A default method depends on an associated type that is not proven at
    /// the declaration hook.
    UnprovenAssociatedType,
    /// A pinned receiver conflicts with the requested async, thread-safe,
    /// catching, or borrowed-output mode.
    PinnedModeConflict,
    /// Invocation was disabled by reflection policy.
    DisabledByPolicy,
}
