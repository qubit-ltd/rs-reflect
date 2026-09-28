// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Invocation validation failures paired with recoverable inputs.

use std::fmt;

use crate::invoke::InvocationError;
use crate::invoke::InvocationMode;
use crate::invoke::InvocationRecovery;

/// A validation error paired with the complete recoverable invocation input.
///
/// # Examples
///
/// ```
/// use qubit_reflect::identity::{FragmentIdentity, MemberId};
/// use qubit_reflect::invoke::{Invocation, ReceiverExpectation};
/// use qubit_reflect::value::Local;
///
/// let identity = MemberId::new(
///     "example::Worker",
///     "run",
///     0,
///     FragmentIdentity::new("example", "example::Worker", 1, 1, "method", 1),
/// );
/// let failure = Invocation::<Local>::associated([])
///     .validate(&identity, ReceiverExpectation::owned::<()>(), &[])
///     .expect_err("an owned receiver is required");
/// assert!(failure.recovery().receiver().is_none());
/// ```
#[must_use]
pub struct InvocationFailure<'call, M: InvocationMode> {
    /// Structured reason the invocation could not enter user code.
    pub(crate) error: InvocationError,
    /// Untouched receiver and arguments retained by the runtime.
    pub(crate) recovery: InvocationRecovery<'call, M>,
}

impl<'call, M: InvocationMode> InvocationFailure<'call, M> {
    /// Returns the structured validation error.
    ///
    /// # Returns
    ///
    /// Returns the reason user code was not called.
    #[must_use = "the invocation error explains why the call failed"]
    #[inline]
    pub const fn error(&self) -> &InvocationError {
        &self.error
    }
    /// Returns the recoverable invocation input.
    ///
    /// # Returns
    ///
    /// Returns the original receiver and arguments.
    #[must_use]
    #[inline]
    pub const fn recovery(&self) -> &InvocationRecovery<'call, M> {
        &self.recovery
    }
    /// Consumes this failure into its error and recovery input.
    ///
    /// # Returns
    ///
    /// Returns the structured error and complete recovery payload.
    #[must_use = "the error and recovery input are required to handle the failure"]
    pub fn into_parts(self) -> (InvocationError, InvocationRecovery<'call, M>) {
        (self.error, self.recovery)
    }
    /// Consumes this failure and returns its recoverable invocation input.
    ///
    /// # Returns
    ///
    /// Returns the original receiver and arguments.
    #[must_use]
    pub fn into_recovery(self) -> InvocationRecovery<'call, M> {
        self.recovery
    }
}

impl<M: InvocationMode> fmt::Debug for InvocationFailure<'_, M> {
    /// Formats the error and recovery metadata without formatting erased
    /// values.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InvocationFailure")
            .field("error", &self.error)
            .field("recovery", &self.recovery)
            .finish()
    }
}

impl<M: InvocationMode> fmt::Display for InvocationFailure<'_, M> {
    /// Delegates the human-readable diagnostic to the structured error.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(formatter)
    }
}

impl<M: InvocationMode> std::error::Error for InvocationFailure<'_, M> {
    /// Returns the structured invocation error as the underlying cause.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}
