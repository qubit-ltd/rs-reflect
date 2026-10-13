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
#[Reflect]
pub struct BoundedConstService<T, const N: usize>(pub T);

#[ReflectImpl]
impl<T, const N: usize> BoundedConstService<T, N>
where
    T: Copy + Into<u64>,
{
    /// Combines bounded generic inputs with the impl's const parameter.
    pub fn score(&self, value: T) -> u64 {
        self.0.into() + value.into() + N as u64
    }
}
