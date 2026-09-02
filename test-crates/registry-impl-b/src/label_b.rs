// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Second independent implementation fragment.

use qubit_reflect::reflect_impl;
use registry_types::RegistryUser;

/// Second external trait used to verify cross-crate impl aggregation.
pub trait LabelB {
    /// Returns the label contributed by implementation fragment B.
    fn label_b(&self) -> u8;
}

#[reflect_impl(external_trait_id = "fixture.registry.label_b")]
impl LabelB for RegistryUser {
    fn label_b(&self) -> u8 {
        self.id + 20
    }
}
