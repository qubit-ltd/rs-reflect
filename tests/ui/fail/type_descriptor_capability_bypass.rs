// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow test-file-name
use qubit_reflect::TypeDescriptor;

fn main() {
    let descriptor = TypeDescriptor::of::<u32>();
    let _ = descriptor.capabilities();
}
