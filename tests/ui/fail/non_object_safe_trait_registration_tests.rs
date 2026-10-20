// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

trait External {
    fn generic<T>(&self, value: T);
}

qubit_reflect::register_trait_object!(dyn External);

fn main() {}
