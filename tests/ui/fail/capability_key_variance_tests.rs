// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::capability::CapabilityKey;
use qubit_reflect::identity::CapabilityId;

fn widen_key(key: CapabilityKey<fn(&str)>) -> CapabilityKey<fn(&'static str)> {
    key
}

fn main() {
    let id = CapabilityId::new("example.ui.capability-key-variance").expect("valid capability ID");
    let key = CapabilityKey::<fn(&str)>::new(id);
    let _ = widen_key(key);
}
