// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::rc::Rc;


#[qubit_reflect::Reflect]
struct Opaque<T> {
    value: T,
}

fn main() {
    let _ = Opaque { value: Rc::new(()) };
}
