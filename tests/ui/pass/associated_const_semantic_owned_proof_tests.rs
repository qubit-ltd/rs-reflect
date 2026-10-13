// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use core::fmt::Debug;


type Bytes = [u8];

#[qubit_reflect::Reflect]
trait AliasedUnsizedConstContract {
    const BYTES: Bytes;
}

#[qubit_reflect::Reflect]
trait MaybeUnsizedConstContract<T: ?Sized> {
    const VALUE: T;
}

#[qubit_reflect::Reflect]
trait LifetimeProjectionConstContract<'a> {
    type Value;
    const VALUE: Self::Value;
}

#[qubit_reflect::Reflect]
trait BorrowedConstContract<'a> {
    const FUNCTION: fn(&'a u8);
    const POINTER: *const &'a u8;
    const REFERENCE: &'a u8;
    const DYN_INPUT: Box<dyn Fn(&'a u8)>;
    const DYN_OUTPUT: Box<dyn Fn() -> &'a u8>;
    const STATIC_FUNCTION: fn(u8) -> u8;
    const STATIC_DYN: Option<Box<dyn Debug>> = None;
}

#[qubit_reflect::Reflect]
trait HrtbConstContract {
    const BOUND_FUNCTION: for<'a> fn(&'a u8);
    const ELIDED_FUNCTION: fn(&u8);
    const BOUND_DYN: Option<Box<dyn for<'a> Fn(&'a u8)>> = None;
    const ELIDED_DYN: Option<Box<dyn Fn(&u8)>> = None;
}

fn main() {}
