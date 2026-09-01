// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::Reflect;
use qubit_reflect::reflect_impl;

#[derive(Reflect)]
struct Worker(u8);

#[reflect_impl]
impl Worker {
    #[reflect(catch_unwind)]
    fn run(&mut self) {
        self.0 += 1;
    }
}

fn main() {}
