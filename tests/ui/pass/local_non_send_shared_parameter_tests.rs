// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::rc::Rc;


#[qubit_reflect::Reflect]
struct Worker;

#[qubit_reflect::ReflectImpl]
impl Worker {
    fn run(value: &Rc<u8>) -> u8 {
        **value
    }
}

fn main() {}
