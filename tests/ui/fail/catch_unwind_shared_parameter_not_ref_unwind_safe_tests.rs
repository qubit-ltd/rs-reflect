// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::cell::Cell;


#[qubit_reflect::Reflect]
#[reflect(opaque)]
struct Fragile {
    state: Cell<u8>,
}

#[qubit_reflect::Reflect]
struct Worker;

#[qubit_reflect::ReflectImpl]
impl Worker {
    #[reflect(catch_unwind)]
    fn run(value: &Fragile) -> u8 {
        value.state.get()
    }
}

fn main() {}
