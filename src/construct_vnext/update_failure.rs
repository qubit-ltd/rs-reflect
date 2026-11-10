// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![allow(clippy::result_large_err)]
use std::fmt;

use super::UpdateError;
use super::UpdateInput;
use super::UpdateRecovery;
use crate::runtime::construct as runtime;
use crate::runtime::error::RecoverableFailure;
use crate::value::DynamicOwned;
use crate::value::ValueMode;

/// Failure from an owned update, retaining the base and every replacement.
///
/// # Type Parameters
///
/// - `M`: Value mode that governs storage and thread-safety bounds.
#[must_use]
pub struct UpdateFailure<M: ValueMode> {
    /// Structured reason the construction or update operation failed.
    error: UpdateError,
    /// Original input retained so its values can be recovered after failure.
    recovery: UpdateInput<M>,
}

impl<M: ValueMode> fmt::Debug for UpdateFailure<M> {
    /// Formats the structured error using its stable debug representation.
    /// # Parameters
    ///
    /// - `formatter`: Formatter receiving the human-readable representation.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("UpdateFailure")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl<M: ValueMode> UpdateFailure<M> {
    /// Wraps a runtime input value produced by the corresponding public
    /// builder. # Parameters
    ///
    /// - `inner`: Runtime representation being adapted to the public API.
    /// # Parameters
    ///
    /// - `inner`: Runtime representation being adapted to the public API.
    pub(super) fn new(inner: runtime::UpdateFailure<M>) -> Self {
        let error = super::update_error::public_update_error(inner.error());
        let recovery = UpdateInput {
            inner: inner.into_input(),
        };
        Self { error, recovery }
    }

    /// Returns the structured update error.
    pub fn error(&self) -> &UpdateError {
        &self.error
    }

    /// Returns the retained base value.
    pub fn base(&self) -> &DynamicOwned<M> {
        &self.recovery.inner.base
    }

    /// Borrows the retained update input.
    pub fn recovery(&self) -> UpdateRecovery<'_, M> {
        UpdateRecovery { input: &self.recovery }
    }

    /// Returns the retained update input.
    pub fn into_input(self) -> UpdateInput<M> {
        self.recovery
    }
}

impl<M: ValueMode> RecoverableFailure for UpdateFailure<M> {
    /// Public error category returned by the recoverable failure contract.
    type Error = UpdateError;
    /// Owned construction or update input retained on failure.
    type Recovery = UpdateInput<M>;

    /// Structured reason the construction or update operation failed.
    fn error(&self) -> &Self::Error {
        &self.error
    }

    /// Original input retained so its values can be recovered after failure.
    fn recovery(&self) -> &Self::Recovery {
        &self.recovery
    }

    /// Separates the structured error from the values retained for recovery.
    fn into_parts(self) -> (Self::Error, Self::Recovery) {
        (self.error, self.recovery)
    }
}
