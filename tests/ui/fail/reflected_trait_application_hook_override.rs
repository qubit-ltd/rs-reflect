// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#[qubit_reflect::Reflect]
trait Hooked<T = u8> {}

struct Target;

#[qubit_reflect::ReflectImpl(reflected_trait = Hooked)]
impl Hooked for Target {
    fn __qbr_trait_application_metadata()
    where
        Self: Sized,
    {
    }
}

fn main() {}
