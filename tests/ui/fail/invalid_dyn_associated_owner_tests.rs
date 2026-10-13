// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================


#[qubit_reflect::Reflect]
trait Parent {
    type Item;
}

#[qubit_reflect::Reflect(supertrait(Parent), dyn_compatible(Fake::Item))]
trait InvalidOwner: Parent {}

fn main() {}
