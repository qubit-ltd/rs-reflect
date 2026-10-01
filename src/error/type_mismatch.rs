// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Errors for dynamic values with an unexpected Rust type.

use std::any::TypeId;

/// A dynamic operation received a value with an unexpected type.
///
/// # Examples
///
/// ```
/// use std::any::TypeId;
/// use qubit_reflect::error::TypeMismatch;
/// let mismatch = TypeMismatch::new(TypeId::of::<u32>(), TypeId::of::<u8>());
/// assert_ne!(mismatch.expected(), mismatch.actual());
/// ```
#[must_use]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, thiserror::Error)]
#[error("dynamic value type did not match the expected type")]
pub struct TypeMismatch {
    /// Runtime type ID required by the operation.
    expected: TypeId,
    /// Runtime type ID supplied by the value.
    actual: TypeId,
    /// Expected type name retained for diagnostics when known.
    expected_name: Option<&'static str>,
    /// Actual type name retained for diagnostics when known.
    actual_name: Option<&'static str>,
}

impl TypeMismatch {
    /// Creates a mismatch from the expected and actual runtime type IDs.
    ///
    /// # Parameters
    ///
    /// - `expected`: Type ID required by the operation.
    /// - `actual`: Type ID supplied by the value.
    ///
    /// # Returns
    ///
    /// Returns a mismatch without optional diagnostic names.
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
    ///
    /// # Parameters
    ///
    /// - `expected_name`: Human-readable name of the required type.
    /// - `actual_name`: Human-readable name of the supplied type.
    ///
    /// # Returns
    ///
    /// Returns this mismatch with both diagnostic names attached.
    pub const fn with_diagnostic_names(mut self, expected_name: &'static str, actual_name: &'static str) -> Self {
        self.expected_name = Some(expected_name);
        self.actual_name = Some(actual_name);
        self
    }

    /// Adds the expected diagnostic type name when the actual erased name is
    /// unavailable.
    ///
    /// # Parameters
    ///
    /// - `expected_name`: Human-readable name of the required type.
    ///
    /// # Returns
    ///
    /// Returns this mismatch with the expected diagnostic name attached.
    pub const fn with_expected_name(mut self, expected_name: &'static str) -> Self {
        self.expected_name = Some(expected_name);
        self
    }

    /// Returns the expected runtime type ID.
    ///
    /// # Returns
    ///
    /// Returns the type ID required by the operation.
    #[must_use]
    #[inline]
    pub const fn expected(&self) -> TypeId {
        self.expected
    }

    /// Returns the actual runtime type ID.
    ///
    /// # Returns
    ///
    /// Returns the type ID supplied by the value.
    #[must_use]
    #[inline]
    pub const fn actual(&self) -> TypeId {
        self.actual
    }

    /// Returns the expected type's diagnostic name when it is available.
    ///
    /// # Returns
    ///
    /// Returns the expected type name, or `None` when no name was retained.
    #[must_use]
    #[inline]
    pub const fn expected_name(&self) -> Option<&'static str> {
        self.expected_name
    }

    /// Returns the actual type's diagnostic name when it is available.
    ///
    /// # Returns
    ///
    /// Returns the actual type name, or `None` when no name was retained.
    #[must_use]
    #[inline]
    pub const fn actual_name(&self) -> Option<&'static str> {
        self.actual_name
    }
}
