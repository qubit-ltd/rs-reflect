// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow explicit-imports
use qubit_reflect as reflect;
use qubit_reflect::Reflect;

#[derive(Reflect)]
struct Record<T> {
    value: T,
}

fn main() {
    let _ = Record { value: 7_u8 };
}
