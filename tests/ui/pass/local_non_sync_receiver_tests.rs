// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::cell::Cell;


#[qubit_reflect::Reflect]
struct Worker {
    state: Cell<u8>,
}

#[qubit_reflect::ReflectImpl]
impl Worker {
    fn run(&self) -> u8 {
        self.state.get()
    }
}

fn main() {}
