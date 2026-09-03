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
mod descriptor;
#[cfg(feature = "derive")]
mod invoke;
mod registry;
mod value;
