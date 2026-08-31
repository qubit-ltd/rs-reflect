// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Tests generated-code compile-time assertion helpers.

use crate::__private::assertions::assert_reflect;
use crate::__private::assertions::assert_static;

/// Verifies reflected static types satisfy both generated-code assertions.
#[test]
fn test_generated_code_assertions_accept_reflected_static_types() {
    assert_reflect::<u8>();
    assert_static::<u8>();
}
