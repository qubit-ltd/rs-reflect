// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Standalone integration target for generic impl trait-link regressions.

#![cfg(feature = "derive")]

#[path = "registry/generic_impl_trait_link_tests.rs"]
mod registry_generic_impl_trait_link_tests;
