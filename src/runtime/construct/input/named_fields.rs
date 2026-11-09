// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Ordered named field values used by construction and update operations.

use super::construction_input::ConstructionInput;
use super::construction_input_value::ConstructionInputValue;
use super::input_entry_ref::InputEntryRef;
use super::into_runtime_owned::IntoRuntimeOwned;
use super::sealed;
use crate::runtime::value::DynamicOwned;
use crate::runtime::value::Local;
use crate::runtime::value::ValueFor;
use crate::runtime::value::ValueMode;

/// Ordered named field values supplied to construction or update.
///
/// # Type Parameters
///
/// * `M` — dynamic value mode used by stored field values; defaults to `Local`.
pub(crate) struct NamedFields<M: ValueMode = Local> {
    /// Field names and owned values in caller-supplied order.
    pub(crate) entries: Vec<(Box<str>, DynamicOwned<M>)>,
}

impl<M: ValueMode> NamedFields<M> {
    /// Creates an empty named field collection.
    pub(crate) fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Appends a concrete reflected value under its source field name.
    ///
    /// # Type Parameters
    ///
    /// * `T` — reflected concrete value type.
    ///
    /// # Parameters
    ///
    /// * `name` — source name of the field being supplied.
    /// * `value` — value moved into the field slot.
    pub(crate) fn field<T: ValueFor<M>>(mut self, name: impl Into<Box<str>>, value: T) -> Self {
        self.entries.push((name.into(), DynamicOwned::new(value)));
        self
    }

    /// Appends an already owned dynamic value under its source field name.
    ///
    /// # Type Parameters
    ///
    /// * `V` — supported runtime or generated dynamic owner.
    ///
    /// # Parameters
    ///
    /// * `name` — source name of the field being supplied.
    /// * `value` — owned dynamic value transferred into the field slot.
    pub(crate) fn field_dynamic<V: IntoRuntimeOwned<M>>(mut self, name: impl Into<Box<str>>, value: V) -> Self {
        self.entries.push((name.into(), value.into_runtime_owned()));
        self
    }

    /// Borrows entries as a name/value view without changing their order.
    pub(crate) fn entries(&self) -> Vec<InputEntryRef<'_, M>> {
        self.entries
            .iter()
            .map(|(name, value)| InputEntryRef::Named { name, value })
            .collect()
    }

    /// Moves the fields into the common construction input representation.
    pub(crate) fn into_input(self) -> ConstructionInputValue<M> {
        ConstructionInputValue::Named(self.entries)
    }
}

impl<M: ValueMode> Default for NamedFields<M> {
    /// Creates an empty collection, matching [`NamedFields::new`].
    fn default() -> Self {
        Self::new()
    }
}

impl<M: ValueMode> ConstructionInput<M> for NamedFields<M> {}
impl<M: ValueMode> sealed::Sealed<M> for NamedFields<M> {
    /// Moves the named entries into the sealed common input representation.
    fn into_input(self) -> ConstructionInputValue<M> {
        ConstructionInputValue::Named(self.entries)
    }
}
