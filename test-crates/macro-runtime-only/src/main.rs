// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Compile fixture for direct derive macros with a runtime-only dependency.

use qubit_reflect_derive::Reflect;
use qubit_reflect_derive::reflect;
use qubit_reflect_derive::reflect_impl;

#[derive(Reflect)]
#[reflect(opaque)]
struct RuntimeOnlyTarget;

#[reflect]
trait RuntimeOnlyTrait {
    #[cfg(any())]
    fn disabled(&self) -> MissingTraitType;
}

#[reflect_impl]
impl RuntimeOnlyTarget {
    #[cfg(any())]
    fn disabled(&self) -> MissingImplType {
        unreachable!()
    }
}

fn main() {}
