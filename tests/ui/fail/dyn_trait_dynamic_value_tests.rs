// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::value::DynamicRef;
use qubit_reflect::value::Local;

#[qubit_reflect::Reflect]
trait Service {
    fn value(&self) -> usize;
}

fn wrap(value: &dyn Service) {
    let _ = DynamicRef::<Local>::new(value);
}

fn main() {}
