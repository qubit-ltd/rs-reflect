// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Retained construction inputs for a failed construction attempt.

use super::super::input::ConstructionInputValue;
use super::super::input::NamedFields;
use super::super::input::PositionalFields;
use super::super::input::UnitFields;
use super::RecoveredConstructionInput;
use crate::runtime::value::ValueMode;

/// Keeps the exact field input available after construction fails.
///
/// # Type Parameters
///
/// * `M` — dynamic value mode used by the retained field values.
#[must_use]
pub(crate) struct ConstructionRecovery<M: ValueMode> {
    /// Original input shape and values retained for inspection or retry.
    pub(super) input: ConstructionInputValue<M>,
}

impl<M: ValueMode> ConstructionRecovery<M> {
    /// Returns the retained input without consuming the recovery handle.
    #[cfg(test)]
    pub(crate) fn input(&self) -> &ConstructionInputValue<M> {
        &self.input
    }

    /// Returns the original input fields in their shape-specific wrapper.
    pub(crate) fn into_input(self) -> RecoveredConstructionInput<M> {
        match self.input {
            ConstructionInputValue::Named(entries) => RecoveredConstructionInput::Named(NamedFields { entries }),
            ConstructionInputValue::Positional(values) => {
                RecoveredConstructionInput::Positional(PositionalFields::from_values(values))
            }
            ConstructionInputValue::Unit => RecoveredConstructionInput::Unit(UnitFields),
        }
    }
}
