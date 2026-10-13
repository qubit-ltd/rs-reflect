// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================


#[qubit_reflect::Reflect]
struct Worker(u8);

#[qubit_reflect::ReflectImpl]
impl Worker {
    #[reflect(catch_unwind)]
    fn run(&mut self) {
        self.0 += 1;
    }
}

fn main() {}
