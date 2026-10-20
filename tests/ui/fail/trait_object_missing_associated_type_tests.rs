// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

trait Parent {
    type Item;
}

qubit_reflect::register_trait_object!(dyn Parent);

fn main() {}
