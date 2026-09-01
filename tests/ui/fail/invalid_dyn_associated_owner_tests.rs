// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::reflect;

#[reflect]
trait Parent {
    type Item;
}

#[reflect(supertrait(Parent), dyn_compatible(Fake::Item))]
trait InvalidOwner: Parent {}

fn main() {}
