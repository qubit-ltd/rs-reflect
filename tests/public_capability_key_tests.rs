// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Public contracts for typed capability keys.

use std::any::TypeId;

use qubit_reflect::capability::CapabilityKey;
use qubit_reflect::identity::CapabilityId;

#[test]
fn test_key_clone_and_debug_preserve_contract() {
    let id = CapabilityId::new("example.test.key").expect("valid capability ID");
    let key = CapabilityKey::<u32>::new(id);
    let clone = Clone::clone(&key);

    assert_eq!(clone.id(), key.id());
    assert_eq!(clone.adapter_type(), TypeId::of::<u32>());
    assert!(format!("{key:?}").contains("CapabilityKey"));
}
