// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Typed pinned invocation components.

use std::fmt;

use super::pinned_ref_invocation_recovery::PinnedRefInvocationRecovery;
use crate::invoke::InvocationError;
use crate::invoke::InvocationMode;

/// A pinned shared invocation validation error and its complete recovery input.
///
/// # Type Parameters
///
/// - `'call`: Lifetime retained by the pinned receiver and arguments.
/// - `T`: Concrete receiver type kept behind the pin.
/// - `M`: Dynamic ownership mode of invocation arguments.
///
/// # Examples
///
/// ```
/// use qubit_reflect::identity::{FragmentIdentity, MemberId};
/// use qubit_reflect::invoke::{ArgumentExpectation, PinnedRefInvocationFailure, PinnedRefInvocation};
/// use qubit_reflect::value::Local;
///
/// let identity = MemberId::new(
///     "example::Worker",
///     "run",
///     0,
///     FragmentIdentity::new("example", "example::Worker", 1, 1, "method", 1),
/// );
/// let value = Box::pin(7_u8);
/// let Err(failure) = PinnedRefInvocation::new(value.as_ref(), [])
///     .validate(&identity, &[ArgumentExpectation::owned::<u8>()])
/// else {
///     panic!("one required argument is missing");
/// };
/// let failure: PinnedRefInvocationFailure<'_, u8, Local> = failure;
/// assert!(failure.error().to_string().contains("expected 1, got 0"));
/// ```
#[must_use]
pub struct PinnedRefInvocationFailure<'call, T: ?Sized, M: InvocationMode> {
    /// Structured reason user code was not called.
    pub(in crate::invoke::pinned) error: InvocationError,
    /// Original pinned receiver and arguments.
    pub(in crate::invoke::pinned) recovery: PinnedRefInvocationRecovery<'call, T, M>,
}

impl<'call, T: ?Sized, M: InvocationMode> PinnedRefInvocationFailure<'call, T, M> {
    /// Returns the structured validation error.
    ///
    /// # Returns
    ///
    /// Returns the reason user code was not called.
    #[must_use = "the invocation error explains why the pinned call failed"]
    #[inline]
    pub const fn error(&self) -> &InvocationError {
        &self.error
    }
    /// Returns the recoverable pinned invocation input.
    ///
    /// # Returns
    ///
    /// Returns the original pinned receiver and arguments.
    #[must_use]
    #[inline]
    pub const fn recovery(&self) -> &PinnedRefInvocationRecovery<'call, T, M> {
        &self.recovery
    }
    /// Consumes this failure into its error and recovery input.
    ///
    /// # Returns
    ///
    /// Returns the structured error and full recovery payload.
    #[must_use = "the error and recovery input are required to handle the failure"]
    pub fn into_parts(self) -> (InvocationError, PinnedRefInvocationRecovery<'call, T, M>) {
        (self.error, self.recovery)
    }
    /// Consumes this failure and returns its recoverable invocation input.
    ///
    /// # Returns
    ///
    /// Returns the original pinned receiver and arguments.
    #[must_use]
    pub fn into_recovery(self) -> PinnedRefInvocationRecovery<'call, T, M> {
        self.recovery
    }
}

impl<T: ?Sized, M: InvocationMode> fmt::Debug for PinnedRefInvocationFailure<'_, T, M> {
    /// Formats validation metadata without requiring erased arguments to
    /// implement `Debug`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PinnedRefInvocationFailure")
            .field("error", &self.error)
            .finish()
    }
}

impl<T: ?Sized, M: InvocationMode> fmt::Display for PinnedRefInvocationFailure<'_, T, M> {
    /// Formats the underlying structured validation error.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(formatter)
    }
}

impl<T: ?Sized, M: InvocationMode> std::error::Error for PinnedRefInvocationFailure<'_, T, M> {
    /// Returns the structured invocation error as the underlying cause.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}
