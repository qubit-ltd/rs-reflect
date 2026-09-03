// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

use std::rc::Rc;

use qubit_reflect::Reflect;

#[derive(Reflect)]
#[reflect(thread_safe)]
struct LocalOnly {
    value: Rc<u8>,
}

fn main() {}
