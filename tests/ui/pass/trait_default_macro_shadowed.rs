// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![allow(dead_code)]

#[allow(unused_macros)]
macro_rules! unreachable {
    ($($tokens:tt)*) => {
        compile_error!("caller macro selected")
    };
}

#[qubit_reflect::Reflect]
trait DefaultContract {
    fn read(&self) -> u32 {
        7
    }
    fn sum(&self, value: u32) -> u32 {
        value + 1
    }
}

#[qubit_reflect::Reflect]
struct Target;

#[qubit_reflect::ReflectImpl(reflected_trait = DefaultContract)]
impl DefaultContract for Target {}

fn main() {}
