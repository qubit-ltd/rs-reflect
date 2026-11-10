// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::NamedFields;
use super::PositionalFields;
use super::UnitFields;
use crate::runtime::construct as runtime;
use crate::value::ValueMode;

/// Recovered construction values, preserving declaration-independent input
/// order.
///
/// # Type Parameters
///
/// - `M`: Value mode that governs storage and thread-safety bounds.
pub enum RecoveredConstructionInput<M: ValueMode> {
    /// Recovered named field values.
    Named(
        /// Builder containing the original name/value pairs.
        NamedFields<M>,
    ),
    /// Recovered positional values in their original order.
    Positional(
        /// Builder containing the original positional values.
        PositionalFields<M>,
    ),
    /// Recovered empty input for a unit target.
    Unit(
        /// Empty builder corresponding to the original unit input.
        UnitFields,
    ),
}

/// Converts runtime recovery values back into the corresponding public builder.
/// # Parameters
///
/// - `input`: Owned values supplied for validation and application.
///
/// # Type Parameters
///
/// - `M`: Value mode that governs storage and thread-safety bounds.
///
/// # Parameters
///
/// - `input`: Owned values supplied for validation and application.
pub(super) fn recovered_input<M: ValueMode>(
    input: runtime::RecoveredConstructionInput<M>,
) -> RecoveredConstructionInput<M> {
    match input {
        runtime::RecoveredConstructionInput::Named(inner) => RecoveredConstructionInput::Named(NamedFields { inner }),
        runtime::RecoveredConstructionInput::Positional(inner) => {
            RecoveredConstructionInput::Positional(PositionalFields { inner })
        }
        runtime::RecoveredConstructionInput::Unit(_) => RecoveredConstructionInput::Unit(UnitFields),
    }
}
