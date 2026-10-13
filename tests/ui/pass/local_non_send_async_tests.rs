// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::cell::Cell;
use std::future::ready;
use std::rc::Rc;


#[qubit_reflect::Reflect]
struct Worker;

#[qubit_reflect::ReflectImpl]
impl Worker {
    async fn run(value: u8) -> u8 {
        let local = Rc::new(Cell::new(value));
        ready(()).await;
        local.get()
    }
}

fn main() {}
