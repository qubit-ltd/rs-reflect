// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================


#[qubit_reflect::Reflect]
#[reflect(opaque)]
struct ConditionalTarget;

#[qubit_reflect::Reflect]
trait ConditionalTrait {
    #[cfg(any())]
    fn disabled(&self) -> MissingTraitType;

    #[cfg(any())]
    #[reflect(rename = "")]
    fn disabled_invalid_helper(&self);

    #[cfg(all())]
    fn enabled(&self) -> u32;
}

#[qubit_reflect::ReflectImpl]
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
