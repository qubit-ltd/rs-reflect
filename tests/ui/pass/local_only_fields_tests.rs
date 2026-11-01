// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::rc::Rc;

#[qubit_reflect::Reflect(local_only)]
struct LocalRecord {
    value: Rc<u8>,
}

#[qubit_reflect::Reflect(local_only)]
enum LocalEvent {
    Value(Rc<u8>),
}

fn main() {
    let _ = LocalEvent::Value(Rc::new(1));
}
