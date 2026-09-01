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
struct Constrained<T>(PhantomData<T>);

#[reflect_impl(specialize(T = String))]
impl<T: Clone + Send> Constrained<T>
where
    T: Sync,
{
}

#[derive(Reflect)]
#[reflect(opaque)]
struct Borrowed<'a, T>(PhantomData<&'a T>);

#[reflect_impl(specialize(T = u8))]
impl<'a, T: 'a> Borrowed<'a, T> {}

fn main() {}
