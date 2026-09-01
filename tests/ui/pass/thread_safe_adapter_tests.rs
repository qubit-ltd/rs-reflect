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
struct Worker;

#[reflect_impl]
impl Worker {
    #[reflect(thread_safe)]
    fn increment(value: u32) -> u32 {
        value + 1
    }
}

fn main() {}
