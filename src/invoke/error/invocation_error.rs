// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Structured invocation validation errors.

use std::fmt;

use crate::identity::MemberId;
use crate::invoke::InvocationErrorKind;

/// A pre-execution invocation validation error with method context.
///
/// Every error is produced before any owned receiver or argument is extracted.
/// The enclosing invocation failure therefore carries complete recovery input.
///
/// # Examples
///
/// ```
/// use qubit_reflect::identity::{FragmentIdentity, MemberId};
/// use qubit_reflect::invoke::{InvocationError, InvocationErrorKind};
///
/// let method = MemberId::new(
///     "example::Worker",
///     "run",
///     0,
///     FragmentIdentity::new("example", "example::Worker", 1, 1, "method", 1),
/// );
/// let error = InvocationError::new(
///     method,
///     InvocationErrorKind::ArgumentCountMismatch { expected: 1, actual: 0 },
/// );
/// assert!(error.to_string().contains("expected 1, got 0"));
/// ```
#[must_use]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InvocationError {
    method_identity: Box<MemberId>,
    kind: Box<InvocationErrorKind>,
}

impl InvocationError {
    /// Creates an invocation error for one exact method member.
    ///
    /// # Parameters
    ///
    /// - `method_identity`: Identity of the method that was rejected.
    /// - `kind`: Machine-readable validation reason.
    ///
    /// # Returns
    ///
    /// Returns a structured invocation error.
    #[must_use = "the invocation error records why this method call was rejected"]
    pub fn new(method_identity: MemberId, kind: InvocationErrorKind) -> Self {
        Self {
            method_identity: Box::new(method_identity),
            kind: Box::new(kind),
        }
    }

    /// Returns the structured identity of the method being invoked.
    ///
    /// # Returns
    ///
    /// Returns the exact method member identity.
    #[must_use]
    #[inline]
    pub fn method_identity(&self) -> &MemberId {
        &self.method_identity
    }

    /// Returns the stable machine-readable validation reason.
    ///
    /// # Returns
    ///
    /// Returns the reason validation rejected the invocation.
    #[must_use]
    #[inline]
    pub const fn kind(&self) -> &InvocationErrorKind {
        &self.kind
    }
}

impl fmt::Display for InvocationError {
    /// Formats method context followed by the non-stable kind diagnostic.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "failed to invoke {} at index {} on `{}`: {}",
            self.method_identity.kind(),
            self.method_identity.index(),
            self.method_identity.declaring_identity(),
            self.kind,
        )
    }
}

impl std::error::Error for InvocationError {
    /// Returns the machine-readable validation kind as the underlying cause.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.kind.as_ref())
    }
}
