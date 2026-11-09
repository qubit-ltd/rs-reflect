// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Consuming input slots used during field binding.

use crate::runtime::value::DynamicOwned;
use crate::runtime::value::ValueMode;

/// An owned named or positional slot awaiting binding to a reflected field.
///
/// # Type Parameters
///
/// * `M` — dynamic value mode of the retained field.
pub(crate) enum InputEntry<M: ValueMode> {
    /// Named slot whose value is taken once during binding.
    Named(
        /// Optional until the binder consumes the value.
        Option<DynamicOwned<M>>,
    ),
    /// Positional slot whose value is taken once during binding.
    Positional(
        /// Optional until the binder consumes the value.
        Option<DynamicOwned<M>>,
    ),
}

impl<M: ValueMode> InputEntry<M> {
    /// Takes the slot value exactly once after binding selected this entry.
    pub(crate) fn take_value(&mut self) -> DynamicOwned<M> {
        match self {
            Self::Named(value) | Self::Positional(value) => value.take().expect("bound input is consumed once"),
        }
    }
}
