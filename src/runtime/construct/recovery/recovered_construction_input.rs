// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Shape-preserving input values returned from failed construction.

use super::super::input::NamedFields;
use super::super::input::PositionalFields;
use super::super::input::UnitFields;
use crate::runtime::value::ValueMode;

/// Original constructor inputs recovered without flattening their field shape.
///
/// # Type Parameters
///
/// * `M` — dynamic value mode used by retained fields.
pub(crate) enum RecoveredConstructionInput<M: ValueMode> {
    /// Named fields supplied by their source names.
    Named(
        /// Retained named entries in their original order.
        NamedFields<M>,
    ),
    /// Positional fields supplied by their ordinal positions.
    Positional(
        /// Retained positional values in their original order.
        PositionalFields<M>,
    ),
    /// Unit construction, which had no field values.
    Unit(
        /// Marker for the absence of constructor inputs.
        UnitFields,
    ),
}
