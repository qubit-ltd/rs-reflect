// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Compile fixture for direct derive macros with a runtime-only dependency.

#[cfg(not(doc))]
mod runtime_only_impl;
#[cfg(not(doc))]
mod runtime_only_target;
#[cfg(not(doc))]
mod runtime_only_trait;

/// Runs the compile fixture for direct derive macros.
fn main() {
    #[cfg(not(doc))]
    runtime_only_trait::assert_runtime_only_trait();
}
