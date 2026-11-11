// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![allow(dead_code, non_camel_case_types, non_upper_case_globals)]

#[qubit_reflect::Reflect]
struct Record<r#type, const r#const: usize> {
    value: r#type,
    bytes: [u8; r#const],
}

fn main() {
    let _ = qubit_reflect::TypeDescriptor::of::<Record<u32, 2>>();
}
