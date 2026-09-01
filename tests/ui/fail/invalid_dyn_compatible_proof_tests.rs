// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::reflect;

trait External {
    fn generic<T>(&self, value: T);
}

#[reflect(
    external_trait(External, id = "example.External"),
    dyn_compatible
)]
trait InvalidProof: External {}

fn main() {}
