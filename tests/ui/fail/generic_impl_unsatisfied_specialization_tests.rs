// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::marker::PhantomData;


#[qubit_reflect::Reflect]
struct Constrained<T>(#[reflect(skip)] PhantomData<T>);

#[qubit_reflect::ReflectImpl(specialize(T = String))]
impl<T: Copy> Constrained<T> {}

fn main() {}
