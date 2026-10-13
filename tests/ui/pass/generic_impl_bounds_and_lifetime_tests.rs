// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::marker::PhantomData;


#[qubit_reflect::Reflect]
#[reflect(opaque)]
struct Constrained<T>(PhantomData<T>);

#[qubit_reflect::ReflectImpl]
impl<T: Clone + Send> Constrained<T>
where
    T: Sync,
{
}

#[qubit_reflect::Reflect]
#[reflect(opaque)]
struct Borrowed<'a, T>(PhantomData<&'a T>);

#[qubit_reflect::ReflectImpl]
impl<'a, T: 'a> Borrowed<'a, T> {}

fn main() {}
