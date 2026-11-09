// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;

/// Public method target whose invocation adapter is bound from another crate.
#[Reflect]
pub struct CrossCrateService(
    /// The service's numeric state.
    pub u8,
);

#[ReflectImpl]
impl CrossCrateService {
    /// Adds a value to the public service state.
    ///
    /// # Parameters
    ///
    /// - `value`: The value to add to the service state.
    ///
    /// # Returns
    ///
    /// The sum of the stored state and the supplied value.
    pub fn add(&self, value: u8) -> u8 {
        self.0 + value
    }
}
