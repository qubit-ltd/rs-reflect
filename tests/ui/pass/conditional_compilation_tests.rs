// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::Reflect;
use qubit_reflect::reflect;
use qubit_reflect::reflect_impl;

#[derive(Reflect)]
#[reflect(opaque)]
struct ConditionalTarget;

#[reflect]
trait ConditionalTrait {
    #[cfg(any())]
    fn disabled(&self) -> MissingTraitType;

    #[cfg(any())]
    #[reflect(rename = "")]
    fn disabled_invalid_helper(&self);

    #[cfg(all())]
    fn enabled(&self) -> u32;
}

#[reflect_impl]
impl ConditionalTarget {
    #[cfg(any())]
    fn disabled(&self) -> MissingImplType {
        unreachable!()
    }

    #[cfg(all())]
    fn enabled(&self) -> u32 {
        1
    }
}

fn main() {}
