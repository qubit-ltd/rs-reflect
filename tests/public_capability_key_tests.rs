// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Public contracts for capability type specifications.

use qubit_reflect::capability::{CapabilitySpec, CloneAdapter, CloneCapability};

fn assert_clone_adapter<C: CapabilitySpec<Adapter = CloneAdapter>>() {}

#[test]
fn built_in_capability_spec_binds_id_and_adapter_type() {
    assert_clone_adapter::<CloneCapability>();
    assert_eq!(CloneCapability::ID, "qubit.reflect.clone");
}
