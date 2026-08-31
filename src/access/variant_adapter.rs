// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Safe function-pointer boundary used by generated enum active tests.

use crate::error::TypeMismatch;
use crate::value::DynamicRef;
use crate::value::Local;

/// Tests whether a variant is active after root enum type validation.
pub type VariantActiveAdapter = for<'a> fn(DynamicRef<'a, Local>) -> Result<bool, TypeMismatch>;
