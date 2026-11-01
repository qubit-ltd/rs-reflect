// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#[qubit_reflect::Reflect(local_only, local_only)]
struct Repeated {
    value: u32,
}

#[qubit_reflect::Reflect(unknown)]
struct Unknown {
    value: u32,
}

#[qubit_reflect::Reflect(local_only)]
trait InvalidTrait {
    fn value(&self);
}

fn main() {}
