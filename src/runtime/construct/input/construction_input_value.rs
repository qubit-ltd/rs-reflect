// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Internal common representation of constructor input shapes.

use super::super::schema::ConstructionShape;
use super::input_entry::InputEntry;
use super::input_entry_ref::InputEntryRef;
use crate::runtime::value::DynamicOwned;
use crate::runtime::value::ValueMode;

/// Owned constructor values represented without losing their input shape.
///
/// # Type Parameters
///
/// * `M` — dynamic value mode of the retained fields.
pub(crate) enum ConstructionInputValue<M: ValueMode> {
    /// Fields supplied by source name.
    Named(
        /// Named values in caller-supplied order.
        Vec<(Box<str>, DynamicOwned<M>)>,
    ),
    /// Fields supplied by tuple position.
    Positional(
        /// Positional values in caller-supplied order.
        Vec<DynamicOwned<M>>,
    ),
    /// Unit input with no values.
    Unit,
}

impl<M: ValueMode> ConstructionInputValue<M> {
    /// Returns the shape needed to validate a construction provider.
    pub(crate) fn shape(&self) -> ConstructionShape {
        match self {
            Self::Named(_) => ConstructionShape::Named,
            Self::Positional(_) => ConstructionShape::Positional,
            Self::Unit => ConstructionShape::Unit,
        }
    }

    /// Borrows each entry while preserving names or positions.
    pub(crate) fn entries(&self) -> Vec<InputEntryRef<'_, M>> {
        match self {
            Self::Named(entries) => entries
                .iter()
                .map(|(name, value)| InputEntryRef::Named { name, value })
                .collect(),
            Self::Positional(values) => values.iter().map(InputEntryRef::Positional).collect(),
            Self::Unit => Vec::new(),
        }
    }

    /// Consumes the input and wraps each value as an unbound slot.
    pub(crate) fn into_entries(self) -> Vec<InputEntry<M>> {
        match self {
            Self::Named(entries) => entries
                .into_iter()
                .map(|(_, value)| InputEntry::Named(Some(value)))
                .collect(),
            Self::Positional(values) => values
                .into_iter()
                .map(|value| InputEntry::Positional(Some(value)))
                .collect(),
            Self::Unit => Vec::new(),
        }
    }
}
