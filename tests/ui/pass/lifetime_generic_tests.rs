// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::Reflect;

#[derive(Reflect)]
struct Borrowed<'a> {
    value: &'a str,
}

fn main() {
    let _ = Borrowed { value: "static" };
    let _ = Borrowed::<'static>::type_descriptor();
}
