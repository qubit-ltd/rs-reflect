// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::ConstructionMode;
use super::ConstructionShape;
use super::NamedFields;
use super::PositionalFields;
use super::RuntimeConstructionInput;
use super::UnitFields;
use crate::runtime::construct::UnitFields as RuntimeUnitFields;

/// Input shape accepted by construction operations.
///
/// # Type Parameters
///
/// - `M`: Value mode that governs storage and thread-safety bounds.
#[doc(hidden)]
pub trait ConstructionInput<M: ConstructionMode> {
    /// Returns the shape of this input.
    fn shape(&self) -> ConstructionShape;

    /// Moves this public input into the sealed runtime validation carrier.
    #[doc(hidden)]
    fn __into_runtime(self) -> RuntimeConstructionInput<M>;
}

impl<M: ConstructionMode> ConstructionInput<M> for NamedFields<M> {
    /// Returns the input layout used to match values with target fields.
    fn shape(&self) -> ConstructionShape {
        ConstructionShape::Named
    }
    /// Moves this public input into the sealed runtime validation carrier.
    fn __into_runtime(self) -> RuntimeConstructionInput<M> {
        RuntimeConstructionInput::new(self.inner.into_input())
    }
}
impl<M: ConstructionMode> ConstructionInput<M> for PositionalFields<M> {
    /// Returns the input layout used to match values with target fields.
    fn shape(&self) -> ConstructionShape {
        ConstructionShape::Positional
    }
    /// Moves this public input into the sealed runtime validation carrier.
    fn __into_runtime(self) -> RuntimeConstructionInput<M> {
        RuntimeConstructionInput::new(self.inner.into_input())
    }
}
impl<M: ConstructionMode> ConstructionInput<M> for UnitFields {
    /// Returns the input layout used to match values with target fields.
    fn shape(&self) -> ConstructionShape {
        ConstructionShape::Unit
    }
    /// Moves this public input into the sealed runtime validation carrier.
    fn __into_runtime(self) -> RuntimeConstructionInput<M> {
        RuntimeConstructionInput::new(RuntimeUnitFields::new().into_input())
    }
}
