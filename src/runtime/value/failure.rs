// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::TypeId;

use crate::runtime::error::RecoverableFailure;
use crate::runtime::error::RecoverableFailureParts;
use crate::runtime::identity::TypeIdentity;

/// Describes the expected and actual types for a rejected dynamic operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeMismatch {
    /// Type identity required by the operation.
    expected: TypeIdentity,
    /// Type identity supplied to the operation.
    actual: TypeIdentity,
}

impl TypeMismatch {
    /// Creates mismatch details from the expected and actual type identities.
    ///
    /// # Parameters
    /// - `expected`: type identity requested by the operation.
    /// - `actual`: type identity of the supplied value.
    pub(crate) fn new(expected: TypeIdentity, actual: TypeIdentity) -> Self {
        Self { expected, actual }
    }

    /// Returns the expected Rust runtime type ID.
    pub fn expected(&self) -> std::any::TypeId {
        self.expected.type_id
    }

    /// Returns the actual Rust runtime type ID.
    pub fn actual(&self) -> std::any::TypeId {
        self.actual.type_id
    }

    /// Returns the expected Rust type name.
    pub fn expected_name(&self) -> &'static str {
        self.expected.rust_name
    }

    /// Returns the actual Rust type name.
    pub fn actual_name(&self) -> &'static str {
        self.actual.rust_name
    }
}

impl std::fmt::Display for TypeMismatch {
    /// Writes both type identities in a concise mismatch description.
    ///
    /// # Parameters
    /// - `formatter`: destination for the formatted error text.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "type mismatch: expected {:?}, got {:?}",
            self.expected, self.actual
        )
    }
}

impl std::error::Error for TypeMismatch {}

/// A failed dynamic downcast that retains the original value for recovery.
///
/// # Type Parameters
/// - `V`: the value type retained for recovery.
#[must_use]
pub struct DowncastFailure<V> {
    /// Expected and actual type identities for the rejected downcast.
    mismatch: TypeMismatch,
    /// Original value retained so the caller can recover it.
    value: V,
}

impl<V> DowncastFailure<V> {
    /// Creates a failure while retaining the value that could not be downcast.
    ///
    /// # Type Parameters
    /// - `V`: the type of the retained value.
    ///
    /// # Parameters
    /// - `expected`: type identity requested by the downcast.
    /// - `actual`: type identity of the retained value.
    /// - `value`: the original value to retain for recovery.
    pub(crate) fn new(expected: TypeIdentity, actual: TypeIdentity, value: V) -> Self {
        Self {
            mismatch: TypeMismatch::new(expected, actual),
            value,
        }
    }

    /// Returns the Rust type ID requested by the downcast.
    pub fn expected(&self) -> TypeId {
        self.mismatch.expected.type_id
    }
    /// Returns the Rust type ID of the retained value.
    pub fn actual(&self) -> TypeId {
        self.mismatch.actual.type_id
    }
    /// Borrows the retained value without consuming the failure.
    pub fn value(&self) -> &V {
        &self.value
    }
    /// Recovers the retained value and discards the mismatch details.
    pub fn into_value(self) -> V {
        self.value
    }
    /// Returns the mismatch details together with the retained value.
    pub fn into_parts(self) -> (TypeMismatch, V) {
        (self.mismatch, self.value)
    }
}

impl<V> std::fmt::Debug for DowncastFailure<V> {
    /// Formats the mismatch and the retained value's type name.
    ///
    /// # Parameters
    /// - `formatter`: destination for the debug representation.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DowncastFailure")
            .field("mismatch", &self.mismatch)
            .field("value", &std::any::type_name::<V>())
            .finish()
    }
}

impl<V> RecoverableFailure for DowncastFailure<V> {
    /// Error information produced by the failed downcast.
    type Error = TypeMismatch;
    /// Retained value returned when the caller recovers from the failure.
    type Recovery = V;

    /// Returns the type mismatch without consuming the failure.
    fn error(&self) -> &Self::Error {
        &self.mismatch
    }

    /// Borrows the retained value for recovery.
    fn recovery(&self) -> &Self::Recovery {
        &self.value
    }

    /// Returns the mismatch and retained value as owned recovery parts.
    fn into_parts(self) -> (Self::Error, Self::Recovery) {
        RecoverableFailureParts::new(self.mismatch, self.value).into_parts()
    }
}
