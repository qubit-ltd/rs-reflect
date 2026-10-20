// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! First independent implementation fragment.

// The derive exports a fixture-only helper macro that is not intended as
// user-facing API and therefore has no public documentation.
#![allow(missing_docs)]

use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use registry_types::RegistryUser;

/// First external trait used to verify cross-crate impl aggregation.
#[Reflect]
pub trait LabelA {
    /// Returns the label contributed by implementation fragment A.
    fn label_a(&self) -> u8;
}

#[ReflectImpl(reflected_trait = LabelA)]
impl LabelA for RegistryUser {
    fn label_a(&self) -> u8 {
        self.id + 10
    }
}
