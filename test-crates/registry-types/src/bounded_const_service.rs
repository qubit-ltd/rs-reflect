// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;

/// A generic dependency type whose method binding depends on a const impl
/// argument.
///
/// # Type Parameters
///
/// - `T`: The value type stored by the service.
/// - `N`: The constant added by the reflected method.
#[Reflect]
pub struct BoundedConstService<T, const N: usize>(
    /// The service's stored value.
    pub T,
);

#[ReflectImpl]
impl<T, const N: usize> BoundedConstService<T, N>
where
    T: Copy + Into<u64>,
{
    /// Combines bounded generic inputs with the impl's const parameter.
    ///
    /// # Parameters
    ///
    /// - `value`: The additional value combined with the stored value.
    ///
    /// # Returns
    ///
    /// The two values plus the const parameter.
    pub fn score(&self, value: T) -> u64 {
        self.0.into() + value.into() + N as u64
    }
}
