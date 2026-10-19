// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::marker::PhantomData;


#[qubit_reflect::Reflect]
struct Missing<T, const N: usize>(PhantomData<[T; N]>);

#[qubit_reflect::ReflectImpl(specialize(T = u8))]
impl<T, const N: usize> Missing<T, N> {}

#[qubit_reflect::Reflect]
struct Duplicate<T, const N: usize>(PhantomData<[T; N]>);

#[qubit_reflect::ReflectImpl(specialize(T = u8, T = u16, N = 4))]
impl<T, const N: usize> Duplicate<T, N> {}

fn main() {}
