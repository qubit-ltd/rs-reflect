// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::marker::PhantomData;

use qubit_reflect::Reflect;
use qubit_reflect::reflect_impl;

#[derive(Reflect)]
#[reflect(opaque)]
struct Missing<T, const N: usize>(PhantomData<[T; N]>);

#[reflect_impl(specialize(T = u8))]
impl<T, const N: usize> Missing<T, N> {}

#[derive(Reflect)]
#[reflect(opaque)]
struct Duplicate<T, const N: usize>(PhantomData<[T; N]>);

#[reflect_impl(specialize(T = u8, T = u16, N = 4))]
impl<T, const N: usize> Duplicate<T, N> {}

fn main() {}
