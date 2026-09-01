// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Ownership recovery for reflected field replacement failures.

use std::fmt;

use crate::access::FieldAccessError;
use crate::access::FieldIdentity;
use crate::value::ReflectedOwned;

/// An untouched replacement retained after field-set validation fails.
pub struct FieldSetRecovery {
    field: FieldIdentity,
    query_name: Option<&'static str>,
    value: ReflectedOwned,
}

impl FieldSetRecovery {
    /// Creates recovery for one field replacement value.
    pub(crate) const fn new(
        field: FieldIdentity,
        query_name: Option<&'static str>,
        value: ReflectedOwned,
    ) -> Self {
        Self {
            field,
            query_name,
            value,
        }
    }

    /// Returns the field whose replacement was rejected.
    #[must_use]
    #[inline(always)]
    pub const fn field(&self) -> &FieldIdentity {
        &self.field
    }

    /// Returns the original query name, or `None` for a positional field.
    #[must_use]
    #[inline(always)]
    pub const fn query_name(&self) -> Option<&'static str> {
        self.query_name
    }

    /// Returns the untouched replacement value.
    #[must_use]
    #[inline(always)]
    pub const fn value(&self) -> &ReflectedOwned {
        &self.value
    }

    /// Returns the replacement when `name` is the field's original query
    /// name.
    ///
    /// `None` means this recovery is positional or belongs to another name.
    pub fn value_by_name(&self, name: &str) -> Option<&ReflectedOwned> {
        (self.query_name == Some(name)).then_some(&self.value)
    }

    /// Returns the replacement when `index` is the field's source index.
    ///
    /// `None` means this recovery belongs to another field position.
    pub fn value_at(&self, index: usize) -> Option<&ReflectedOwned> {
        (self.field.index() == index).then_some(&self.value)
    }

    /// Consumes recovery and returns the untouched replacement value.
    pub fn into_value(self) -> ReflectedOwned {
        self.value
    }

    /// Takes the replacement by its original query name without panicking.
    ///
    /// Returns the intact recovery when `name` does not match or the field is
    /// positional.
    pub fn into_value_by_name(
        self,
        name: &str,
    ) -> Result<ReflectedOwned, Self> {
        if self.query_name == Some(name) {
            Ok(self.value)
        } else {
            Err(self)
        }
    }

    /// Takes the replacement by its source index without panicking.
    ///
    /// Returns the intact recovery when `index` does not match.
    pub fn into_value_at(self, index: usize) -> Result<ReflectedOwned, Self> {
        if self.field.index() == index {
            Ok(self.value)
        } else {
            Err(self)
        }
    }
}

impl fmt::Debug for FieldSetRecovery {
    /// Formats binding metadata without requiring the erased value to be
    /// `Debug`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FieldSetRecovery")
            .field("field", &self.field)
            .field("query_name", &self.query_name)
            .field("value", &"<value>")
            .finish()
    }
}

/// A field-set error with recovery when validation rejected the input before
/// an adapter ran.
///
/// Adapter errors occur after the adapter accepts ownership and therefore do
/// not contain recovery. Call [`Self::recovery`] to distinguish the two
/// phases without inspecting display text.
pub struct FieldSetFailure {
    error: Box<FieldAccessError>,
    recovery: Option<Box<FieldSetRecovery>>,
}

impl FieldSetFailure {
    /// Creates a pre-execution failure retaining the untouched replacement.
    pub(crate) fn before_execution(
        error: FieldAccessError,
        field: FieldIdentity,
        query_name: Option<&'static str>,
        value: ReflectedOwned,
    ) -> Self {
        Self {
            error: Box::new(error),
            recovery: Some(Box::new(FieldSetRecovery::new(
                field, query_name, value,
            ))),
        }
    }

    /// Creates an adapter failure after ownership crossed the execution
    /// boundary.
    pub(crate) fn after_execution(error: FieldAccessError) -> Self {
        Self {
            error: Box::new(error),
            recovery: None,
        }
    }

    /// Returns the machine-readable field access error.
    #[must_use]
    #[inline(always)]
    pub const fn error(&self) -> &FieldAccessError {
        &self.error
    }

    /// Returns the untouched replacement for a pre-execution failure.
    ///
    /// `None` means an adapter already accepted ownership before it reported
    /// the error.
    #[must_use]
    #[inline(always)]
    pub fn recovery(&self) -> Option<&FieldSetRecovery> {
        self.recovery.as_deref()
    }

    /// Consumes the failure and returns its error and optional recovery.
    pub fn into_parts(self) -> (FieldAccessError, Option<FieldSetRecovery>) {
        (*self.error, self.recovery.map(|recovery| *recovery))
    }

    /// Consumes the failure and returns pre-execution recovery.
    ///
    /// Returns the structured adapter error when execution already accepted
    /// ownership and recovery is therefore unavailable.
    pub fn into_recovery(self) -> Result<FieldSetRecovery, FieldAccessError> {
        match self.recovery {
            Some(recovery) => Ok(*recovery),
            None => Err(*self.error),
        }
    }

    /// Consumes the failure and returns its machine-readable error.
    pub fn into_error(self) -> FieldAccessError {
        *self.error
    }
}

impl fmt::Debug for FieldSetFailure {
    /// Formats the error and recovery metadata without formatting erased
    /// values.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FieldSetFailure")
            .field("error", &self.error)
            .field("recovery", &self.recovery)
            .finish()
    }
}

impl fmt::Display for FieldSetFailure {
    /// Delegates human-readable output to the structured access error.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(formatter)
    }
}

impl std::error::Error for FieldSetFailure {
    /// Returns the underlying machine-readable access error.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.error.as_ref())
    }
}

impl AsRef<FieldAccessError> for FieldSetFailure {
    /// Borrows the underlying access error for compatibility with generic
    /// error inspection code.
    fn as_ref(&self) -> &FieldAccessError {
        &self.error
    }
}
