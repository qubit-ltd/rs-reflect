// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#[qubit_reflect::Reflect]
struct Target;

trait UnreflectedTrait {
    fn value(&self) -> u8;
}

#[qubit_reflect::ReflectImpl(reflected_trait = UnreflectedTrait)]
impl UnreflectedTrait for Target {
    fn value(&self) -> u8 {
        1
    }
}

fn main() {}
