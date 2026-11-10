// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![allow(clippy::result_large_err)]
use std::fmt;

use super::ConstructionError;
use super::ConstructionRecovery;
use super::RecoveredConstructionInput;
use crate::runtime::construct as runtime;
use crate::runtime::error::RecoverableFailure;
use crate::value::ValueMode;

/// Construction failure with recoverable owned input.
///
/// # Type Parameters
///
/// - `M`: Value mode that governs storage and thread-safety bounds.
#[must_use]
pub struct ConstructionFailure<M: ValueMode> {
    /// Structured reason the construction or update operation failed.
    error: ConstructionError,
    /// Original input retained so its values can be recovered after failure.
    recovery: RecoveredConstructionInput<M>,
}

impl<M: ValueMode> fmt::Debug for ConstructionFailure<M> {
    /// Formats the structured error using its stable debug representation.
    /// # Parameters
    ///
    /// - `formatter`: Formatter receiving the human-readable representation.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ConstructionFailure")
            .field("error", &self.error)
            .field("shape", &self.recovery().shape())
            .finish()
    }
}

impl<M: ValueMode> ConstructionFailure<M> {
    /// Wraps a runtime input value produced by the corresponding public
    /// builder. # Parameters
    ///
    /// - `inner`: Runtime representation being adapted to the public API.
    /// # Parameters
    ///
    /// - `inner`: Runtime representation being adapted to the public API.
    pub(super) fn new(inner: runtime::ConstructionFailure<M>) -> Self {
        let error = super::construction_error::public_construction_error(inner.error());
        let recovery = super::recovered_construction_input::recovered_input(inner.into_input());
        Self { error, recovery }
    }

    /// Returns the structured construction error.
    pub fn error(&self) -> &ConstructionError {
        &self.error
    }

    /// Recovers the original input shape and values.
    pub fn recovery(&self) -> ConstructionRecovery<'_, M> {
        ConstructionRecovery { input: &self.recovery }
    }

    /// Recovers the original input shape and values.
    pub fn into_input(self) -> RecoveredConstructionInput<M> {
        self.recovery
    }
}
impl<M: ValueMode> RecoverableFailure for ConstructionFailure<M> {
    /// Public error category returned by the recoverable failure contract.
    type Error = ConstructionError;
    /// Owned construction or update input retained on failure.
    type Recovery = RecoveredConstructionInput<M>;

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
