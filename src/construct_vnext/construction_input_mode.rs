// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::ConstructionMode;
use super::NamedFields;
use super::PositionalFields;
use super::UnitFields;
use crate::value::Local;

/// The value mode associated with a sealed construction input type.
#[doc(hidden)]
pub trait ConstructionInputMode {
    /// Storage mode used by the input values.
    type Mode: ConstructionMode;
}

impl<M: ConstructionMode> ConstructionInputMode for NamedFields<M> {
    /// Value mode used to store this input’s values.
    type Mode = M;
}
impl<M: ConstructionMode> ConstructionInputMode for PositionalFields<M> {
    /// Value mode used to store this input’s values.
    type Mode = M;
}
impl ConstructionInputMode for UnitFields {
    /// Value mode used to store this input’s values.
    type Mode = Local;
}
