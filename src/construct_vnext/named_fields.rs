// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::runtime::construct::NamedFields as RuntimeNamedFields;
use crate::value::DynamicOwned;
use crate::value::Local;
use crate::value::ValueFor;
use crate::value::ValueMode;

/// Named values supplied to struct-like construction or update.
///
/// Values remain owned by this builder until construction succeeds or a
/// failure returns them for recovery.
///
/// # Type Parameters
///
/// - `M`: Value storage mode, local by default.
pub struct NamedFields<M: ValueMode = Local> {
    /// Runtime collection that retains the named values in insertion order.
    pub(super) inner: RuntimeNamedFields<M>,
}

impl<M: ValueMode> NamedFields<M> {
    /// Creates an empty named input.
    pub fn new() -> Self {
        Self {
            inner: RuntimeNamedFields::new(),
        }
    }

    /// Adds a typed value under `name`; validation is deferred until
    /// construction.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete value type accepted by the selected value mode.
    ///
    /// # Parameters
    ///
    /// - `name`: Reflected field name to match against the target.
    /// - `value`: Owned value retained for the construction attempt.
    ///
    /// # Returns
    ///
    /// Returns this builder with the named value appended.
    pub fn field<T: ValueFor<M>>(mut self, name: impl Into<Box<str>>, value: T) -> Self {
        self.inner = self.inner.field(name, value);
        self
    }

    /// Adds a dynamically typed owned value under `name`.
    ///
    /// # Parameters
    ///
    /// - `name`: Reflected field name to match against the target.
    /// - `value`: Dynamic value retained for validation and construction.
    ///
    /// # Returns
    ///
    /// Returns this builder with the named value appended.
    pub fn field_dynamic(mut self, name: impl Into<Box<str>>, value: DynamicOwned<M>) -> Self {
        self.inner = self.inner.field_dynamic(name, value);
        self
    }
}

impl<M: ValueMode> Default for NamedFields<M> {
    /// Creates an empty field input that can be populated with builder methods.
    fn default() -> Self {
        Self::new()
    }
}
