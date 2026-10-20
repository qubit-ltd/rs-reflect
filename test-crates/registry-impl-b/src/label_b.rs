// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Second independent implementation fragment.

// The derive exports a fixture-only helper macro that is not intended as
// user-facing API and therefore has no public documentation.
#![allow(missing_docs)]

use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use registry_types::RegistryUser;

/// Second external trait used to verify cross-crate impl aggregation.
#[Reflect]
pub trait LabelB {
    /// Returns the label contributed by implementation fragment B.
    fn label_b(&self) -> u8;
}

#[ReflectImpl(reflected_trait = LabelB)]
impl LabelB for RegistryUser {
    fn label_b(&self) -> u8 {
        self.id + 20
    }
}
