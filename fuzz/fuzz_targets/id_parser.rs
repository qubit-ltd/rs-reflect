// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![no_main]

use libfuzzer_sys::fuzz_target;
use qubit_reflect::identity::CapabilityId;
use qubit_reflect::identity::ExternalTraitId;

const MAX_INPUT_BYTES: usize = 4_096;

// Exercises bounded public identifier parsing with arbitrary lossy UTF-8 input.
fuzz_target!(|unbounded: &[u8]| {
    let data = &unbounded[..unbounded.len().min(MAX_INPUT_BYTES)];
    let data = String::from_utf8_lossy(data);
    let capability_id = CapabilityId::validate(&data);
    let repeated_capability_id = CapabilityId::validate(&data);
    assert_eq!(capability_id, repeated_capability_id);

    let external_trait_id = ExternalTraitId::new(&data);
    let repeated_external_trait_id = ExternalTraitId::new(&data);
    assert_eq!(external_trait_id, repeated_external_trait_id);
});
