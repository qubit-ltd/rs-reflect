// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Integration tests for reflection registry APIs.

mod cross_crate_tests;
mod model_facade_tests;
mod runtime_tests;
#[cfg(feature = "derive")]
mod stress_tests;
