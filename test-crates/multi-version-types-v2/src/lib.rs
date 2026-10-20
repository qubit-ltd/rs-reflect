// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! The second version fixture for same-short-name reflected identity tests.

use qubit_reflect::Reflect;

#[Reflect]
/// A reflected type with the same short name as the v1 fixture.
pub struct SameName;
