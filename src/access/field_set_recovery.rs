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
use crate::value::DynamicOwned;
use crate::value::Local;
use crate::value::Mode;

/// An untouched replacement retained after field-set validation fails.
///
/// # Type Parameters
///
/// - `M`: Local or thread-safe ownership mode of the retained replacement.
///
/// # Examples
///
/// ```
/// #[cfg(feature = "derive")]
/// fn main() {
///     use qubit_reflect::ReflectedMut;
///     use qubit_reflect::ReflectedOwned;
///     use qubit_reflect::TypeDescriptor;
///     use qubit_reflect::access::FieldSetRecovery;
///     use qubit_reflect::value::Local;
///     mod example {
///         use qubit_reflect::Reflect;
///         #[derive(Reflect)]
///         #[reflect(crate = qubit_reflect)]
///         pub struct Record {
///             #[reflect(read_only)]
///             pub value: u32,
///         }
///     }
///     let field = TypeDescriptor::of::<example::Record>().field("value").expect("field exists");
///     let mut record = example::Record { value: 7 };
///     let failure = field
///         .set(ReflectedMut::new(&mut record), ReflectedOwned::new(9_u32))
///         .expect_err("the field is read-only");
///     let recovery: &FieldSetRecovery<Local> = failure.recovery().expect("input is retained");
///     assert_eq!(recovery.value().downcast_ref::<u32>(), Some(&9));
/// }
/// #[cfg(not(feature = "derive"))]
/// fn main() {}
/// ```
#[must_use]
pub struct FieldSetRecovery<M: Mode = Local> {
    field: FieldIdentity,
    query_name: Option<&'static str>,
    value: DynamicOwned<M>,
}

impl<M: Mode> FieldSetRecovery<M> {
    /// Creates recovery for one field replacement value.
    ///
    /// # Parameters
    ///
    /// - `field`: Identity of the field whose set operation was rejected.
    /// - `query_name`: Original lookup name, or `None` for a positional field.
    /// - `value`: Untouched replacement value retained for retry or inspection.
    ///
    /// # Returns
    ///
    /// Recovery metadata that still owns the original replacement.
    pub(crate) const fn new(field: FieldIdentity, query_name: Option<&'static str>, value: DynamicOwned<M>) -> Self {
        Self {
            field,
            query_name,
            value,
        }
    }

    /// Returns the field whose replacement was rejected.
    ///
    /// # Returns
    ///
    /// The identity used to address the rejected field.
    #[must_use]
    #[inline]
    pub const fn field(&self) -> &FieldIdentity {
        &self.field
    }

    /// Returns the original query name, or `None` for a positional field.
    ///
    /// # Returns
    ///
    /// The original query name when the field was addressed by name.
    #[must_use]
    #[inline]
    pub const fn query_name(&self) -> Option<&'static str> {
        self.query_name
    }

    /// Returns the untouched replacement value.
    ///
    /// # Returns
    ///
    /// A shared borrow of the value retained for recovery.
    #[must_use]
    #[inline]
    pub const fn value(&self) -> &DynamicOwned<M> {
        &self.value
    }

    /// Returns the replacement when `name` is the field's original query
    /// name.
    ///
    /// `None` means this recovery is positional or belongs to another name.
    ///
    /// # Parameters
    ///
    /// - `name`: Query name to match against the original field lookup.
    ///
    /// # Returns
    ///
    /// The retained value when the name matches; otherwise `None`.
    #[must_use]
    pub fn value_by_name(&self, name: &str) -> Option<&DynamicOwned<M>> {
        (self.query_name == Some(name)).then_some(&self.value)
    }

    /// Returns the replacement when `index` is the field's source index.
    ///
    /// `None` means this recovery belongs to another field position.
    ///
    /// # Parameters
    ///
    /// - `index`: Zero-based field index to match.
    ///
    /// # Returns
    ///
    /// The retained value when the index matches; otherwise `None`.
    #[must_use]
    pub fn value_at(&self, index: usize) -> Option<&DynamicOwned<M>> {
        (self.field.index() == index).then_some(&self.value)
    }

    /// Consumes recovery and returns the untouched replacement value.
    ///
    /// # Returns
    ///
    /// The original value, transferring its ownership to the caller.
    #[must_use]
    #[inline]
    pub fn into_value(self) -> DynamicOwned<M> {
        self.value
    }

    /// Takes the replacement by its original query name without panicking.
    ///
    /// Returns the intact recovery when `name` does not match or the field is
    /// positional.
    ///
    /// # Parameters
    ///
    /// - `name`: Query name that must match the original field lookup.
    ///
    /// # Returns
    ///
    /// The replacement on a match, or the intact recovery on a mismatch.
    pub fn into_value_by_name(self, name: &str) -> Result<DynamicOwned<M>, Self> {
        if self.query_name == Some(name) {
            Ok(self.value)
        } else {
            Err(self)
        }
    }

    /// Takes the replacement by its source index without panicking.
    ///
    /// Returns the intact recovery when `index` does not match.
    ///
    /// # Parameters
    ///
    /// - `index`: Zero-based field index that must match.
    ///
    /// # Returns
    ///
    /// The replacement on a match, or the intact recovery on a mismatch.
    pub fn into_value_at(self, index: usize) -> Result<DynamicOwned<M>, Self> {
        if self.field.index() == index {
            Ok(self.value)
        } else {
            Err(self)
        }
    }
}

impl<M: Mode> fmt::Debug for FieldSetRecovery<M> {
    /// Formats binding metadata without requiring the erased value to be
    /// `Debug`.
    ///
    /// # Parameters
    ///
    /// - `formatter`: Destination receiving the recovery metadata.
    ///
    /// # Returns
    ///
    /// The formatter result after writing field and query-name metadata.
    ///
    /// # Errors
    ///
    /// Returns the formatter's error if the destination rejects the output.
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
///
/// # Type Parameters
///
/// - `M`: Local or thread-safe ownership mode of any retained replacement.
///
/// # Examples
///
/// ```
/// #[cfg(feature = "derive")]
/// fn main() {
///     use qubit_reflect::ReflectedMut;
///     use qubit_reflect::ReflectedOwned;
///     use qubit_reflect::TypeDescriptor;
///     use qubit_reflect::access::FieldSetFailure;
///     mod example {
///         use qubit_reflect::Reflect;
///         #[derive(Reflect)]
///         #[reflect(crate = qubit_reflect)]
///         pub struct Record {
///             #[reflect(read_only)]
///             pub value: u32,
///         }
///     }
///     let field = TypeDescriptor::of::<example::Record>().field("value").expect("field exists");
///     let mut record = example::Record { value: 7 };
///     let failure: FieldSetFailure = field
///         .set(ReflectedMut::new(&mut record), ReflectedOwned::new(9_u32))
///         .expect_err("the field is read-only");
///     assert!(failure.error().to_string().contains("read-only"));
///     assert!(failure.recovery().is_some());
/// }
/// #[cfg(not(feature = "derive"))]
/// fn main() {}
/// ```
#[must_use]
pub struct FieldSetFailure<M: Mode = Local> {
    error: Box<FieldAccessError>,
    recovery: Option<Box<FieldSetRecovery<M>>>,
}

impl<M: Mode> FieldSetFailure<M> {
    /// Creates a pre-execution failure retaining the untouched replacement.
    ///
    /// # Parameters
    ///
    /// - `error`: Validation error produced before the adapter runs.
    /// - `field`: Identity of the field that rejected the replacement.
    /// - `query_name`: Original query name, or `None` for positional access.
    /// - `value`: Replacement value retained for recovery.
    ///
    /// # Returns
    ///
    /// A failure that owns both the error and untouched replacement.
    pub(crate) fn before_execution(
        error: FieldAccessError,
        field: FieldIdentity,
        query_name: Option<&'static str>,
        value: DynamicOwned<M>,
    ) -> Self {
        Self {
            error: Box::new(error),
            recovery: Some(Box::new(FieldSetRecovery::new(field, query_name, value))),
        }
    }

    /// Creates an adapter failure after ownership crossed the execution
    /// boundary.
    ///
    /// # Parameters
    ///
    /// - `error`: Error reported after the adapter accepted the replacement.
    ///
    /// # Returns
    ///
    /// A failure without pre-execution recovery data.
    pub(crate) fn after_execution(error: FieldAccessError) -> Self {
        Self {
            error: Box::new(error),
            recovery: None,
        }
    }

    /// Returns the machine-readable field access error.
    ///
    /// # Returns
    ///
    /// The structured reason the field-set operation failed.
    #[inline]
    pub const fn error(&self) -> &FieldAccessError {
        &self.error
    }

    /// Returns the untouched replacement for a pre-execution failure.
    ///
    /// `None` means an adapter already accepted ownership before it reported
    /// the error.
    ///
    /// # Returns
    ///
    /// The untouched replacement for validation failures, or `None` after the
    /// adapter accepted ownership.
    #[must_use]
    #[inline]
    pub fn recovery(&self) -> Option<&FieldSetRecovery<M>> {
        self.recovery.as_deref()
    }

    /// Consumes the failure and returns its error and optional recovery.
    ///
    /// # Returns
    ///
    /// The structured error and any value retained before adapter execution.
    pub fn into_parts(self) -> (FieldAccessError, Option<FieldSetRecovery<M>>) {
        (*self.error, self.recovery.map(|recovery| *recovery))
    }

    /// Consumes the failure and returns pre-execution recovery.
    ///
    /// Returns the structured adapter error when execution already accepted
    /// ownership and recovery is therefore unavailable.
    ///
    /// # Returns
    ///
    /// Pre-execution recovery when available.
    ///
    /// # Errors
    ///
    /// Returns the adapter error when execution already accepted ownership.
    pub fn into_recovery(self) -> Result<FieldSetRecovery<M>, FieldAccessError> {
        match self.recovery {
            Some(recovery) => Ok(*recovery),
            None => Err(*self.error),
        }
    }

    /// Consumes the failure and returns its machine-readable error.
    ///
    /// # Returns
    ///
    /// The structured reason for the failed operation.
    pub fn into_error(self) -> FieldAccessError {
        *self.error
    }
}

impl<M: Mode> fmt::Debug for FieldSetFailure<M> {
    /// Formats the error and recovery metadata without formatting erased
    /// values.
    ///
    /// # Parameters
    ///
    /// - `formatter`: Destination receiving the diagnostic representation.
    ///
    /// # Returns
    ///
    /// The formatter result after writing error and recovery metadata.
    ///
    /// # Errors
    ///
    /// Returns the formatter's error if the destination rejects the output.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FieldSetFailure")
            .field("error", &self.error)
            .field("recovery", &self.recovery)
            .finish()
    }
}

impl<M: Mode> fmt::Display for FieldSetFailure<M> {
    /// Delegates human-readable output to the structured access error.
    ///
    /// # Parameters
    ///
    /// - `formatter`: Destination receiving the error description.
    ///
    /// # Returns
    ///
    /// The formatter result returned by the underlying error.
    ///
    /// # Errors
    ///
    /// Returns the formatter's error if the destination rejects the output.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(formatter)
    }
}

impl<M: Mode> std::error::Error for FieldSetFailure<M> {
    /// Returns the underlying machine-readable access error.
    ///
    /// # Returns
    ///
    /// The structured field-access error that caused this failure.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.error.as_ref())
    }
}

impl<M: Mode> AsRef<FieldAccessError> for FieldSetFailure<M> {
    /// Borrows the underlying access error for compatibility with generic
    /// error inspection code.
    ///
    /// # Returns
    ///
    /// A borrow of the underlying field-access error.
    fn as_ref(&self) -> &FieldAccessError {
        &self.error
    }
}
