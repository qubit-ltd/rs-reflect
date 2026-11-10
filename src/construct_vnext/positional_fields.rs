// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::runtime::construct::PositionalFields as RuntimePositionalFields;
use crate::value::DynamicOwned;
use crate::value::Local;
use crate::value::ValueFor;
use crate::value::ValueMode;

/// Positional values supplied to tuple-like construction.
///
/// Values are retained in insertion order and matched to fields by position.
///
/// # Type Parameters
///
/// - `M`: Value storage mode, local by default.
pub struct PositionalFields<M: ValueMode = Local> {
    /// Runtime collection that retains positional values in insertion order.
    pub(super) inner: RuntimePositionalFields<M>,
}

impl<M: ValueMode> PositionalFields<M> {
    /// Creates an empty positional input.
    pub fn new() -> Self {
        Self {
            inner: RuntimePositionalFields::new(),
        }
    }

    /// Appends a typed value at the next positional field index.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete value type accepted by the selected value mode.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned positional value retained for construction.
    ///
    /// # Returns
    ///
    /// Returns this builder with the value appended.
    pub fn value<T: ValueFor<M>>(mut self, value: T) -> Self {
        self.inner = self.inner.value(value);
        self
    }

    /// Appends a dynamically typed value at the next positional field index.
    ///
    /// # Parameters
    ///
    /// - `value`: Dynamic value retained for validation and construction.
    ///
    /// # Returns
    ///
    /// Returns this builder with the value appended.
    pub fn value_dynamic(mut self, value: DynamicOwned<M>) -> Self {
        self.inner = self.inner.value_dynamic(value);
        self
    }
}

impl<M: ValueMode> Default for PositionalFields<M> {
    /// Creates an empty field input that can be populated with builder methods.
    fn default() -> Self {
        Self::new()
    }
}
