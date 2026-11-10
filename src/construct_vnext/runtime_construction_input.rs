// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::runtime::construct as runtime;
use crate::value::ValueMode;

/// Opaque bridge used by the public input contract to reach runtime validation.
///
/// # Type Parameters
///
/// - `M`: Value mode that governs storage and thread-safety bounds.
#[doc(hidden)]
pub struct RuntimeConstructionInput<M: ValueMode> {
    /// Runtime input storage that owns the values until construction succeeds
    /// or failure recovery.
    inner: runtime::input::ConstructionInputValue<M>,
}

impl<M: ValueMode> RuntimeConstructionInput<M> {
    /// Wraps a runtime input value produced by the corresponding public
    /// builder. # Parameters
    ///
    /// - `inner`: Runtime representation being adapted to the public API.
    /// # Parameters
    ///
    /// - `inner`: Runtime representation being adapted to the public API.
    pub(super) fn new(inner: runtime::input::ConstructionInputValue<M>) -> Self {
        Self { inner }
    }
}

impl<M: runtime::ConstructionMode> runtime::input::sealed::Sealed<M> for RuntimeConstructionInput<M> {
    /// Transfers the retained input values into the construction runtime.
    fn into_input(self) -> runtime::input::ConstructionInputValue<M> {
        self.inner
    }
}
impl<M: runtime::ConstructionMode> runtime::ConstructionInput<M> for RuntimeConstructionInput<M> {}
