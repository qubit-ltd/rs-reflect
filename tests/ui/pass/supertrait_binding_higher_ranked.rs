// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::Reflect;

#[Reflect]
trait Parent<T> {}

#[Reflect(supertrait_binding(0 => Parent))]
trait Child: for<'a> Parent<&'a str> {}

fn main() {}
