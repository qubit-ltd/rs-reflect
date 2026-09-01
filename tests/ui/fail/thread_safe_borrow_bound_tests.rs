// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::rc::Rc;

use qubit_reflect::SendReflectedRef;

fn main() {
    let value = Rc::new(1_u8);
    let _ = SendReflectedRef::new(&value);
}
