// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Parent trait shared by the cross-crate registry fixtures.

use qubit_reflect::reflect;

/// A dependency-owned dyn-compatible trait with an inherited binding.
#[reflect]
pub trait RegistryDynParent {
    /// The value exposed by the parent trait.
    type Item;

    /// Returns the parent value.
    fn parent(&self) -> Self::Item;
}
