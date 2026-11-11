// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::Reflect;
use qubit_reflect::TypeDescriptor;

#[Reflect]
pub struct Record {
    pub count: u32,
}

fn main() {
    assert!(TypeDescriptor::of::<Record>().field("count").is_ok());
}
