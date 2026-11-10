// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::UpdateInput;
use crate::value::DynamicOwned;
use crate::value::ValueMode;

/// Recovery view for an update failure.
///
/// # Type Parameters
///
/// - `'a`: Lifetime of the borrowed input or view.
/// - `M`: Value mode that governs storage and thread-safety bounds.
pub struct UpdateRecovery<'a, M: ValueMode> {
    /// Borrowed recovered input retained by the failure object.
    pub(super) input: &'a UpdateInput<M>,
}

impl<M: ValueMode> UpdateRecovery<'_, M> {
    /// Returns the preserved base value.
    pub fn base(&self) -> &DynamicOwned<M> {
        &self.input.inner.base
    }
}
