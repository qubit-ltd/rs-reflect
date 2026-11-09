// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Input marker for unit struct and unit variant construction.

use super::construction_input::ConstructionInput;
use super::construction_input_value::ConstructionInputValue;
use super::sealed;
use crate::runtime::value::ValueMode;

/// Marks the input shape for unit structs and unit enum variants.
pub(crate) struct UnitFields;

impl UnitFields {
    /// Creates the marker for a unit-shaped construction request.
    pub(crate) const fn new() -> Self {
        Self
    }

    /// Converts the marker to the common input representation for mode `M`.
    ///
    /// # Type Parameters
    ///
    /// * `M` — dynamic value mode of the common input representation.
    pub(crate) fn into_input<M: ValueMode>(self) -> ConstructionInputValue<M> {
        ConstructionInputValue::Unit
    }
}

impl<M: ValueMode> ConstructionInput<M> for UnitFields {}
impl<M: ValueMode> sealed::Sealed<M> for UnitFields {
    /// Converts the marker to the sealed unit input representation.
    fn into_input(self) -> ConstructionInputValue<M> {
        ConstructionInputValue::Unit
    }
}
