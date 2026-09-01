// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Errors for dynamic values with an unexpected Rust type.

use std::any::TypeId;

/// A dynamic operation received a value with an unexpected type.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, thiserror::Error)]
#[error("dynamic value type did not match the expected type")]
pub struct TypeMismatch {
    expected: TypeId,
    actual: TypeId,
    expected_name: Option<&'static str>,
    actual_name: Option<&'static str>,
}

impl TypeMismatch {
    /// Creates a mismatch from the expected and actual runtime type IDs.
    pub const fn new(expected: TypeId, actual: TypeId) -> Self {
        Self {
            expected,
            actual,
            expected_name: None,
            actual_name: None,
        }
    }

    /// Adds diagnostic type names without changing the type IDs used for
    /// matching.
    pub const fn with_diagnostic_names(
        mut self,
        expected_name: &'static str,
        actual_name: &'static str,
    ) -> Self {
        self.expected_name = Some(expected_name);
        self.actual_name = Some(actual_name);
        self
    }
    /// Returns the expected runtime type ID.
    #[must_use]
    #[inline(always)]
    pub const fn expected(&self) -> TypeId {
        self.expected
    }
    /// Returns the actual runtime type ID.
    #[must_use]
    #[inline(always)]
    pub const fn actual(&self) -> TypeId {
        self.actual
    }
    /// Returns the expected type's diagnostic name when it is available.
    #[must_use]
    #[inline(always)]
    pub const fn expected_name(&self) -> Option<&'static str> {
        self.expected_name
    }
    /// Returns the actual type's diagnostic name when it is available.
    #[must_use]
    #[inline(always)]
    pub const fn actual_name(&self) -> Option<&'static str> {
        self.actual_name
    }
}
