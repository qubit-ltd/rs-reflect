// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! First independent implementation fragment.

use qubit_reflect::reflect_impl;
use registry_types::RegistryUser;

/// First external trait used to verify cross-crate impl aggregation.
pub trait LabelA {
    /// Returns the label contributed by implementation fragment A.
    fn label_a(&self) -> u8;
}

#[reflect_impl(external_trait_id = "fixture.registry.label_a")]
impl LabelA for RegistryUser {
    fn label_a(&self) -> u8 {
        self.id + 10
    }
}
