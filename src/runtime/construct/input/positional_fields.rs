// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Ordered positional field values used by tuple construction.

use super::construction_input::ConstructionInput;
use super::construction_input_value::ConstructionInputValue;
use super::into_runtime_owned::IntoRuntimeOwned;
use super::sealed;
use crate::runtime::value::DynamicOwned;
use crate::runtime::value::Local;
use crate::runtime::value::ValueFor;
use crate::runtime::value::ValueMode;

/// Ordered positional field values supplied to tuple construction.
///
/// # Type Parameters
///
/// * `M` — dynamic value mode used by stored fields; defaults to `Local`.
pub(crate) struct PositionalFields<M: ValueMode = Local> {
    /// Owned values in the caller-supplied positional order.
    pub(super) values: Vec<DynamicOwned<M>>,
}

impl<M: ValueMode> PositionalFields<M> {
    /// Rebuilds the positional wrapper from values retained after failure.
    ///
    /// # Parameters
    ///
    /// * `values` — original owned values in positional order.
    pub(crate) fn from_values(values: Vec<DynamicOwned<M>>) -> Self {
        Self { values }
    }

    /// Creates an empty positional field collection.
    pub(crate) fn new() -> Self {
        Self { values: Vec::new() }
    }

    /// Appends a concrete reflected value to the next tuple position.
    ///
    /// # Type Parameters
    ///
    /// * `T` — reflected concrete value type.
    ///
    /// # Parameters
    ///
    /// * `value` — value moved into the next positional slot.
    pub(crate) fn value<T: ValueFor<M>>(mut self, value: T) -> Self {
        self.values.push(DynamicOwned::new(value));
        self
    }

    /// Appends an already owned dynamic value to the next tuple position.
    ///
    /// # Type Parameters
    ///
    /// * `V` — supported runtime or generated dynamic owner.
    ///
    /// # Parameters
    ///
    /// * `value` — owned dynamic value transferred into the next slot.
    pub(crate) fn value_dynamic<V: IntoRuntimeOwned<M>>(mut self, value: V) -> Self {
        self.values.push(value.into_runtime_owned());
        self
    }

    /// Moves the values into the common construction input representation.
    pub(crate) fn into_input(self) -> ConstructionInputValue<M> {
        ConstructionInputValue::Positional(self.values)
    }
}

impl<M: ValueMode> Default for PositionalFields<M> {
    /// Creates an empty collection, matching [`PositionalFields::new`].
    fn default() -> Self {
        Self::new()
    }
}

impl<M: ValueMode> ConstructionInput<M> for PositionalFields<M> {}
impl<M: ValueMode> sealed::Sealed<M> for PositionalFields<M> {
    /// Moves the positional values into the sealed common input representation.
    fn into_input(self) -> ConstructionInputValue<M> {
        ConstructionInputValue::Positional(self.values)
    }
}
