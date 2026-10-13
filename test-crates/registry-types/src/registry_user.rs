// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Types shared by the cross-crate registry fixtures.

use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;

/// A reflected type shared by the cross-crate registry fixtures.
#[Reflect]
pub struct RegistryUser {
    /// The identifier used by registry aggregation assertions.
    pub id: u8,
}

/// Public method target whose invocation adapter is bound from another crate.
#[Reflect]
pub struct CrossCrateService(pub u8);

#[ReflectImpl]
impl CrossCrateService {
    /// Adds a value to the public service state.
    pub fn add(&self, value: u8) -> u8 {
        self.0 + value
    }
}
