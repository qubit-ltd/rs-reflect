// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::Reflect;

#[derive(Reflect)]
enum Event<T> {
    Value(T),
    Empty,
}

fn main() {
    let _ = Event::Value(7_u8);
    let _ = Event::<String>::Empty;
    let _ = Event::<u8>::type_descriptor();
}
