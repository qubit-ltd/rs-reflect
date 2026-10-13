// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

trait Reflect: 'static {
    // empty
}

struct ShadowValue;

impl Reflect for ShadowValue {}

#[qubit_reflect::Reflect]
struct Target;

#[qubit_reflect::Reflect]
trait ShadowBoundContract {
    type Item: crate::Reflect + 'static;
}

#[qubit_reflect::ReflectImpl]
impl ShadowBoundContract for Target {
    type Item = ShadowValue;
}

fn main() {}
