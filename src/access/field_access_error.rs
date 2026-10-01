// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Structured errors raised before or during reflected field access.

use crate::access::FieldAccessOperation;
use crate::access::FieldIdentity;
use crate::error::TypeMismatch;

/// A checked field access failed before producing or changing a field value.
///
/// # Examples
///
/// ```
/// use std::any::TypeId;
///
/// use qubit_reflect::access::FieldAccessError;
/// use qubit_reflect::access::FieldIdentity;
///
/// let error = FieldAccessError::InactiveVariant {
///     field: FieldIdentity::new(TypeId::of::<u32>(), "u32", 0, Some("value")),
/// };
/// assert!(error.to_string().contains("u32::value"));
/// ```
#[must_use]
#[derive(Clone, Debug, Eq, Hash, PartialEq, thiserror::Error)]
pub enum FieldAccessError {
    /// The dynamic target is not the field's declaring root type.
    #[error("target type mismatch while accessing {field}")]
    TargetTypeMismatch {
        /// The field whose operation was requested.
        field: FieldIdentity,
        /// The exact expected and actual target identities.
        mismatch: Box<TypeMismatch>,
    },
    /// A replacement value is not the field's exact declared type.
    #[error("replacement value type mismatch while setting {field}")]
    ValueTypeMismatch {
        /// The field whose operation was requested.
        field: FieldIdentity,
        /// The exact expected and actual value identities.
        mismatch: Box<TypeMismatch>,
    },
    /// A skipped field does not expose any dynamic access adapter.
    #[error("field {field} is skipped and does not support {operation}")]
    Skipped {
        /// The field whose operation was requested.
        field: FieldIdentity,
        /// The rejected operation.
        operation: FieldAccessOperation,
    },
    /// A read-only field rejected a mutable operation.
    #[error("field {field} is read-only and does not support {operation}")]
    ReadOnly {
        /// The field whose operation was requested.
        field: FieldIdentity,
        /// The rejected mutable operation.
        operation: FieldAccessOperation,
    },
    /// Structural descriptor data exists but no adapter supports the operation.
    #[error("field {field} has no adapter for {operation}")]
    Unavailable {
        /// The field whose operation was requested.
        field: FieldIdentity,
        /// The unavailable operation.
        operation: FieldAccessOperation,
    },
    /// The field belongs to a different currently inactive enum variant.
    #[error("field {field} belongs to an inactive enum variant")]
    InactiveVariant {
        /// The field whose operation was requested.
        field: FieldIdentity,
    },
}

impl FieldAccessError {
    /// Creates an inactive-variant error for a generated enum-field adapter.
    ///
    /// The adapter calls this only after the descriptor has validated the root
    /// enum type. The returned error does not modify the target.
    ///
    /// # Parameters
    ///
    /// - `field`: Identity of the field belonging to the inactive variant.
    ///
    /// # Returns
    ///
    /// An error identifying the inactive variant field.
    #[doc(hidden)]
    pub const fn inactive_variant(field: FieldIdentity) -> Self {
        Self::InactiveVariant { field }
    }

    /// Returns the field identity shared by every error classification.
    ///
    /// # Returns
    ///
    /// The field identity retained by this error.
    #[must_use]
    #[inline]
    pub const fn field(&self) -> &FieldIdentity {
        match self {
            Self::TargetTypeMismatch { field, .. }
            | Self::ValueTypeMismatch { field, .. }
            | Self::Skipped { field, .. }
            | Self::ReadOnly { field, .. }
            | Self::Unavailable { field, .. }
            | Self::InactiveVariant { field, .. } => field,
        }
    }
}
