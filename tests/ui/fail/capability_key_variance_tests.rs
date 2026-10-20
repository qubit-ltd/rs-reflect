// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::Reflect;
use qubit_reflect::register_capability;
use qubit_reflect::capability::CloneCapability;

#[Reflect]
struct NotClone;

register_capability!(NotClone: CloneCapability);

fn main() {
    // The registration macro must reject a Clone adapter unsupported by the target type.
}
