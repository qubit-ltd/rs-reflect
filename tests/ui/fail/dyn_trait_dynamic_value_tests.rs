// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::ReflectedRef;
use qubit_reflect::reflect;

#[reflect]
trait Service {
    fn value(&self) -> usize;
}

fn wrap(value: &dyn Service) {
    let _ = ReflectedRef::new(value);
}

fn main() {}
