// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::marker::PhantomData;

use qubit_reflect::TypeDescriptor;
use qubit_reflect::descriptor::Reflect as ReflectTrait;

trait Element {
    type Tail: ReflectTrait;
}

struct Bytes;

impl Element for Bytes {
    type Tail = u8;
}

#[qubit_reflect::Reflect]
struct Buffer<T, const N: usize>(PhantomData<[T; N]>);

#[qubit_reflect::ReflectImpl]
impl<T: Element, const N: usize> Buffer<T, N> {
    fn tail_path(value: <T as Element>::Tail) -> <T as Element>::Tail {
        value
    }

    fn const_array(value: [u8; N]) -> [u8; N] {
        value
    }
}

fn main() {
    assert_eq!(
        TypeDescriptor::of::<Buffer<Bytes, 4>>()
            .definition()
            .expect("Buffer retains its source declaration")
            .name(),
        "Buffer"
    );
}
