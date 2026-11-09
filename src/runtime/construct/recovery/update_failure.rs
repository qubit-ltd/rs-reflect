// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Errors and recoverable inputs from a failed reflected update.

use std::fmt;

use super::super::input::UpdateInput;
use super::super::schema::UpdateError;
use super::UpdateRecovery;
use crate::runtime::error::RecoverableFailure;
use crate::runtime::error::RecoverableFailureParts;
use crate::runtime::value::ValueMode;

/// An update error paired with the original base and replacement values.
///
/// # Type Parameters
///
/// * `M` — dynamic value mode used by the retained values.
#[must_use]
pub(crate) struct UpdateFailure<M: ValueMode> {
    /// Reason the requested update could not be completed.
    error: UpdateError,
    /// Base and replacements retained for inspection or retry.
    recovery: UpdateRecovery<M>,
}

impl<M: ValueMode> UpdateFailure<M> {
    /// Associates an update error with the input that triggered it.
    ///
    /// # Parameters
    ///
    /// * `error` — reason the update stopped.
    /// * `input` — base and replacement fields supplied to the update.
    pub(crate) fn new(error: UpdateError, input: UpdateInput<M>) -> Self {
        Self {
            error,
            recovery: UpdateRecovery { input },
        }
    }

    /// Borrows the reason the update failed.
    pub(crate) fn error(&self) -> &UpdateError {
        &self.error
    }

    /// Consumes the failure and returns the original base and replacement
    /// fields.
    pub(crate) fn into_input(self) -> UpdateInput<M> {
        self.recovery.into_input()
    }
}

impl<M: ValueMode> fmt::Debug for UpdateFailure<M> {
    /// Formats the error and base type ID without exposing value contents.
    ///
    /// # Parameters
    ///
    /// * `formatter` — destination for the debug representation.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("UpdateFailure")
            .field("error", &self.error)
            .field("base_type_id", &self.recovery.input.base.type_id())
            .finish()
    }
}

impl<M: ValueMode> RecoverableFailure for UpdateFailure<M> {
    /// Update error reported to the caller.
    type Error = UpdateError;
    /// Original base and replacement inputs retained for retry.
    type Recovery = UpdateRecovery<M>;

    /// Returns the update error retained by this failure.
    fn error(&self) -> &Self::Error {
        &self.error
    }

    /// Borrows the original update recovery handle.
    fn recovery(&self) -> &Self::Recovery {
        &self.recovery
    }

    /// Separates the error from the retained update inputs.
    fn into_parts(self) -> (Self::Error, Self::Recovery) {
        RecoverableFailureParts::new(self.error, self.recovery).into_parts()
    }
}
