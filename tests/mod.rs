// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Unified integration-test entry point for `qubit-reflect`.

mod access;
#[cfg(feature = "derive")]
mod construct;
#[cfg(feature = "derive")]
#[path = "derive_examples/derive_examples_tests.rs"]
mod derive_examples_tests;
mod descriptor;
mod invoke;
mod public_api_accessors_tests;
mod public_capability_key_tests;
mod registry;
mod value;
