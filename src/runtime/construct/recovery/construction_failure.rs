// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Errors and recoverable inputs from a failed reflected construction.

use std::fmt;

use super::super::input::ConstructionInputValue;
use super::super::schema::ConstructionError;
use super::ConstructionRecovery;
use super::RecoveredConstructionInput;
use crate::runtime::error::RecoverableFailure;
use crate::runtime::error::RecoverableFailureParts;
use crate::runtime::value::ValueMode;

/// A construction error paired with the original inputs needed for recovery.
///
/// # Type Parameters
///
/// * `M` — dynamic value mode used by the retained constructor inputs.
#[must_use]
pub(crate) struct ConstructionFailure<M: ValueMode> {
    /// Reason the requested value could not be constructed.
    error: ConstructionError,
    /// Original constructor fields retained for inspection or retry.
    recovery: ConstructionRecovery<M>,
}

impl<M: ValueMode> ConstructionFailure<M> {
    /// Associates a construction error with the input that triggered it.
    ///
    /// # Parameters
    ///
    /// * `error` — reason construction stopped.
    /// * `input` — original shape and field values supplied to construction.
    pub(crate) fn new(error: ConstructionError, input: ConstructionInputValue<M>) -> Self {
        Self {
            error,
            recovery: ConstructionRecovery { input },
        }
    }

    /// Borrows the reason construction failed.
    pub(crate) fn error(&self) -> &ConstructionError {
        &self.error
    }

    /// Borrows the retained input for test inspection.
    #[cfg(test)]
    pub(crate) fn recovery(&self) -> &ConstructionRecovery<M> {
        &self.recovery
    }

    /// Consumes the failure and returns the original shape-specific inputs.
    pub(crate) fn into_input(self) -> RecoveredConstructionInput<M> {
        self.recovery.into_input()
    }
}

impl<M: ValueMode> fmt::Debug for ConstructionFailure<M> {
    /// Formats the error and input shape without exposing field values.
    ///
    /// # Parameters
    ///
    /// * `formatter` — destination for the debug representation.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ConstructionFailure")
            .field("error", &self.error)
            .field("shape", &self.recovery.input.shape())
            .finish()
    }
}

impl<M: ValueMode> RecoverableFailure for ConstructionFailure<M> {
    /// Construction error reported to the caller.
    type Error = ConstructionError;
    /// Original named, positional, or unit-shaped constructor inputs.
    type Recovery = ConstructionRecovery<M>;

    /// Returns the construction error retained by this failure.
    fn error(&self) -> &Self::Error {
        &self.error
    }

    /// Borrows the original input recovery handle.
    fn recovery(&self) -> &Self::Recovery {
        &self.recovery
    }

    /// Separates the error from the retained constructor inputs.
    fn into_parts(self) -> (Self::Error, Self::Recovery) {
        RecoverableFailureParts::new(self.error, self.recovery).into_parts()
    }
}
