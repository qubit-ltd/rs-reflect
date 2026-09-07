// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

//! Bounded transactional snapshot construction through the public API.
#![no_main]

use libfuzzer_sys::fuzz_target;

#[path = "../../tests/internal/registry_snapshot_model_tests.rs"]
mod registry_snapshot_model;

fuzz_target!(|data: &[u8]| registry_snapshot_model::check(data));
