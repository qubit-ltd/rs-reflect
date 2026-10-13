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
    async fn run() -> u8 {
        let local = Rc::new(1_u8);
        std::future::pending::<()>().await;
        *local
    }
}

fn main() {}
