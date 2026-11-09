// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Owned base values and named replacements for struct update.

use super::into_runtime_owned::IntoRuntimeOwned;
use super::named_fields::NamedFields;
use crate::runtime::value::DynamicOwned;
use crate::runtime::value::Local;
use crate::runtime::value::ValueFor;
use crate::runtime::value::ValueMode;

/// An owned base value and ordered field replacements for struct update.
///
/// # Type Parameters
///
/// * `M` — dynamic value mode used by the base and replacements; defaults to
///   `Local`.
pub(crate) struct UpdateInput<M: ValueMode = Local> {
    /// Base object to update.
    pub(crate) base: DynamicOwned<M>,
    /// Ordered named replacement values.
    pub(crate) replacements: NamedFields<M>,
}

impl<M: ValueMode> UpdateInput<M> {
    /// Creates an update request by erasing a reflected base value.
    ///
    /// # Type Parameters
    ///
    /// * `T` — reflected concrete base type.
    ///
    /// # Parameters
    ///
    /// * `base` — value whose selected fields will be replaced.
    pub(crate) fn new<T: ValueFor<M>>(base: T) -> Self {
        Self::from_dynamic(DynamicOwned::new(base))
    }

    /// Creates an update request from an already owned dynamic base value.
    ///
    /// # Type Parameters
    ///
    /// * `V` — supported runtime or generated dynamic owner.
    ///
    /// # Parameters
    ///
    /// * `base` — owned value whose selected fields will be replaced.
    pub(crate) fn from_dynamic<V: IntoRuntimeOwned<M>>(base: V) -> Self {
        Self {
            base: base.into_runtime_owned(),
            replacements: NamedFields::new(),
        }
    }

    /// Appends a concrete reflected replacement under its source field name.
    ///
    /// # Type Parameters
    ///
    /// * `T` — reflected replacement value type.
    ///
    /// # Parameters
    ///
    /// * `name` — source field name to replace.
    /// * `value` — replacement moved into the update request.
    pub(crate) fn field<T: ValueFor<M>>(mut self, name: impl Into<Box<str>>, value: T) -> Self {
        self.replacements = self.replacements.field(name, value);
        self
    }

    /// Appends an already owned dynamic replacement under its source field
    /// name.
    ///
    /// # Type Parameters
    ///
    /// * `V` — supported runtime or generated dynamic owner.
    ///
    /// # Parameters
    ///
    /// * `name` — source field name to replace.
    /// * `value` — owned replacement transferred into the update request.
    pub(crate) fn field_dynamic<V: IntoRuntimeOwned<M>>(mut self, name: impl Into<Box<str>>, value: V) -> Self {
        self.replacements = self.replacements.field_dynamic(name, value);
        self
    }
}
