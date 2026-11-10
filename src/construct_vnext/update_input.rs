// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::runtime::construct::UpdateInput as RuntimeUpdateInput;
use crate::value::DynamicOwned;
use crate::value::Local;
use crate::value::ValueFor;
use crate::value::ValueMode;

/// Owned base value plus its named replacements.
///
/// # Type Parameters
///
/// - `M`: Value mode that governs storage and thread-safety bounds.
pub struct UpdateInput<M: ValueMode = Local> {
    /// Runtime input storage that owns the values until construction succeeds
    /// or failure recovery.
    pub(super) inner: RuntimeUpdateInput<M>,
}

impl<M: ValueMode> UpdateInput<M> {
    /// Starts an update from a typed owned value.
    ///
    /// # Parameters
    ///
    /// - `base`: Owned base value whose fields may be replaced.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    ///
    ///
    /// # Parameters
    ///
    /// - `base`: Owned base value whose fields may be replaced.
    pub fn new<T: ValueFor<M>>(base: T) -> Self {
        Self {
            inner: RuntimeUpdateInput::new(base),
        }
    }

    /// Starts an update from a dynamic owned value.
    ///
    /// # Parameters
    ///
    /// - `base`: Owned base value whose fields may be replaced.
    pub fn from_dynamic(base: DynamicOwned<M>) -> Self {
        Self {
            inner: RuntimeUpdateInput::from_dynamic(base),
        }
    }

    /// Adds a typed replacement.
    ///
    /// # Parameters
    ///
    /// - `name`: Reflected field name to match against the target.
    /// - `value`: Owned value retained for validation and construction.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    ///
    ///
    /// # Parameters
    ///
    /// - `name`: Reflected field name to match against the target.
    /// - `value`: Owned value retained for validation and construction.
    pub fn field<T: ValueFor<M>>(mut self, name: impl Into<Box<str>>, value: T) -> Self {
        self.inner = self.inner.field(name, value);
        self
    }

    /// Adds a dynamically owned replacement.
    ///
    /// # Parameters
    ///
    /// - `name`: Reflected field name to match against the target.
    /// - `value`: Owned value retained for validation and construction.
    pub fn field_dynamic(mut self, name: impl Into<Box<str>>, value: DynamicOwned<M>) -> Self {
        self.inner = self.inner.field_dynamic(name, value);
        self
    }
}
