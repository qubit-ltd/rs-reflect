// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================


#[qubit_reflect::Reflect]
struct Service;

#[qubit_reflect::Reflect]
trait ReflectedService {
    fn value(&self) -> u8;
}

#[qubit_reflect::ReflectImpl(external_trait_id = "invalid.reflected.service")]
impl ReflectedService for Service {
    fn value(&self) -> u8 {
        1
    }
}

fn main() {}
