// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================


#[qubit_reflect::Reflect]
struct Worker;

#[qubit_reflect::ReflectImpl]
impl Worker {
    #[reflect(catch_unwind)]
    async fn run() {}
}

fn main() {}
