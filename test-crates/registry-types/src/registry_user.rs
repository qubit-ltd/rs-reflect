// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Types shared by the cross-crate registry fixtures.

use qubit_reflect::Reflect;

/// A reflected type shared by the cross-crate registry fixtures.
#[derive(Reflect)]
pub struct RegistryUser {
    /// The identifier used by registry aggregation assertions.
    pub id: u8,
}
