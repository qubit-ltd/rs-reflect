// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use core::fmt::Debug;

use qubit_reflect::reflect;

#[reflect]
trait UnsizedAssociatedConstContract {
    const SLICE: [u8];
    const TRAIT_OBJECT: dyn Debug;
    const BOXED_TRAIT_OBJECT: Option<Box<dyn Debug>> = None;
}

fn main() {}
