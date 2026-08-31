// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::reflect;
use qubit_reflect::reflect_impl;

trait Reflect: 'static {}

struct ShadowValue;

impl Reflect for ShadowValue {}

#[derive(qubit_reflect::Reflect)]
struct Target;

#[reflect]
trait ShadowBoundContract {
    type Item: crate::Reflect + 'static;
}

#[reflect_impl]
impl ShadowBoundContract for Target {
    type Item = ShadowValue;
}

fn main() {}
