// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::reflect;
use qubit_reflect::reflect_impl;
use qubit_reflect::Reflect;

#[derive(Reflect)]
#[reflect(opaque)]
struct Service;

#[reflect]
trait ReflectedService {
    fn value(&self) -> u8;
}

#[reflect_impl(external_trait_id = "invalid.reflected.service")]
impl ReflectedService for Service {
    fn value(&self) -> u8 {
        1
    }
}

fn main() {}
