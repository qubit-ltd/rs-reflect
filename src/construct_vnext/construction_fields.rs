// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::ConstructField;

/// A view of the declaration-order fields used by construction and update.
#[derive(Clone, Copy)]
pub struct ConstructionFields<'a> {
    /// Field metadata borrowed from the target's construction plan.
    fields: &'a [ConstructField],
}

impl<'a> ConstructionFields<'a> {
    /// Creates a field view over metadata owned by a construction descriptor.
    ///
    /// # Parameters
    ///
    /// - `fields`: Field metadata exposed by the target descriptor in
    ///   declaration order.
    pub(super) fn new(fields: &'a [ConstructField]) -> Self {
        Self { fields }
    }

    /// Returns the number of declared construction fields.
    pub fn len(self) -> usize {
        self.fields.len()
    }

    /// Returns whether the target has no fields.
    pub fn is_empty(self) -> bool {
        self.fields.is_empty()
    }

    /// Returns field metadata at a declaration-order index without allocation.
    ///
    /// # Parameters
    ///
    /// - `index`: Zero-based declaration-order field index.
    ///
    /// # Returns
    ///
    /// Returns `Some` with the borrowed field metadata when `index` is in
    /// range, or `None` when it is outside the target's fields.
    pub fn get(self, index: usize) -> Option<&'a ConstructField> {
        self.fields.get(index)
    }

    /// Iterates over fields in declaration order.
    pub fn iter(self) -> std::slice::Iter<'a, ConstructField> {
        self.fields.iter()
    }
}
