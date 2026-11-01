// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::marker::PhantomData;

#[qubit_reflect::Reflect]
struct Borrowed<'a> {
    value: &'a str,
    #[reflect(skip)]
    marker: PhantomData<&'a ()>,
}

#[qubit_reflect::Reflect]
enum BorrowedEvent<'a> {
    Value(&'a str),
    #[reflect(skip)]
    Marker(PhantomData<&'a ()>),
}

fn main() {
    let _ = Borrowed {
        value: "x",
        marker: PhantomData,
    };
}
