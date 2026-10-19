// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================


#[qubit_reflect::Reflect]
struct DefaultStrSample;

#[qubit_reflect::Reflect]
trait DefaultStr {
    fn echo_default(value: &str) -> &str {
        value
    }
}

#[qubit_reflect::ReflectImpl]
impl DefaultStr for DefaultStrSample {}

fn main() {}
