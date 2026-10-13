// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::rc::Rc;

use qubit_reflect::value::DynamicOwned;
use qubit_reflect::value::Local;

fn main() {
    let value = DynamicOwned::<Local>::new(Rc::new(7_u8));
    std::thread::spawn(move || drop(value));
}
