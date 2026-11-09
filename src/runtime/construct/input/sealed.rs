// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Private sealing trait that consumes supported construction inputs.

use super::construction_input_value::ConstructionInputValue;
use crate::runtime::value::ValueMode;

/// Converts a supported input builder to the common owned representation.
///
/// # Type Parameters
///
/// * `M` — dynamic value mode of the retained values.
pub(crate) trait Sealed<M: ValueMode> {
    /// Consumes the builder while retaining its named, positional, or unit
    /// shape.
    fn into_input(self) -> ConstructionInputValue<M>;
}
