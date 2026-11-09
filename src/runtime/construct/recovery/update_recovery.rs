// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Retained base value and replacements from a failed update attempt.

use super::super::input::UpdateInput;
#[cfg(test)]
use crate::runtime::value::DynamicOwned;
use crate::runtime::value::ValueMode;

/// Keeps the base and replacements available after an update fails.
///
/// # Type Parameters
///
/// * `M` — dynamic value mode used by the retained base and replacements.
#[must_use]
pub(crate) struct UpdateRecovery<M: ValueMode> {
    /// Original base and replacement entries supplied to the update.
    pub(super) input: UpdateInput<M>,
}

impl<M: ValueMode> UpdateRecovery<M> {
    /// Borrows the base value retained for test inspection.
    #[cfg(test)]
    pub(crate) fn base(&self) -> &DynamicOwned<M> {
        &self.input.base
    }

    /// Returns the original base and replacement entries.
    pub(crate) fn into_input(self) -> UpdateInput<M> {
        self.input
    }
}
