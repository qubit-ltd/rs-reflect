// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::ConstructionShape;
use super::RecoveredConstructionInput;
use crate::value::ValueMode;

/// Recoverable construction input view.
///
/// # Type Parameters
///
/// - `'a`: Lifetime of the borrowed input or view.
/// - `M`: Value mode that governs storage and thread-safety bounds.
pub struct ConstructionRecovery<'a, M: ValueMode> {
    /// Borrowed recovered input retained by the failure object.
    pub(super) input: &'a RecoveredConstructionInput<M>,
}

impl<M: ValueMode> ConstructionRecovery<'_, M> {
    /// Returns the shape retained for recovery.
    pub fn shape(&self) -> ConstructionShape {
        match self.input {
            RecoveredConstructionInput::Named(_) => ConstructionShape::Named,
            RecoveredConstructionInput::Positional(_) => ConstructionShape::Positional,
            RecoveredConstructionInput::Unit(_) => ConstructionShape::Unit,
        }
    }
}
