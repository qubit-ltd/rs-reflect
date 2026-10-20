// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Parent trait shared by the cross-crate registry fixtures.

// The derive exports a fixture-only helper macro that is not intended as
// user-facing API and therefore has no public documentation.
#![allow(missing_docs)]

use qubit_reflect::Reflect;

/// A dependency-owned dyn-compatible trait with an inherited binding.
#[Reflect]
pub trait RegistryDynParent {
    /// The value exposed by the parent trait.
    type Item;

    /// Returns the parent value.
    fn parent(&self) -> Self::Item;

    /// Returns a concrete default value to exercise cross-crate adapters.
    fn default_value(&self) -> u8 {
        9
    }
}
