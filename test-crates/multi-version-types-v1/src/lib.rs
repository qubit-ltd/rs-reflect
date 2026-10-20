// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Types used to verify reflected identity across versioned dependencies.

use qubit_reflect::Reflect;

#[Reflect]
/// A reflected type with the same crate and type name as the v2 fixture.
pub struct SameName;
