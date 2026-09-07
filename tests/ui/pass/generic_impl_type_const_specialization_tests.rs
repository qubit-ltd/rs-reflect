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

trait Element {
    type Tail: Reflect;
}

struct Bytes;

impl Element for Bytes {
    type Tail = u8;
}

#[derive(Reflect)]
#[reflect(opaque)]
struct Buffer<T, const N: usize>(PhantomData<[T; N]>);

#[reflect_impl(specialize(T = Bytes, N = 4))]
impl<T: Element, const N: usize> Buffer<T, N> {
    fn tail_path(value: <T as Element>::Tail) -> <T as Element>::Tail {
        value
    }

    fn const_array(value: [u8; N]) -> [u8; N] {
        value
    }
}

fn main() {
    let _ = Buffer::<Bytes, 4>::type_descriptor();
}
