// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Errors returned by borrowed optional value projection.

use crate::error::TypeMismatch;

/// Reports why an optional value could not be projected.
///
/// # Examples
///
/// ```
/// use qubit_reflect::{OptionalProjectionError, ReflectedRef, TypeDescriptor};
///
/// let optional = TypeDescriptor::of::<Option<u32>>()
///     .as_optional()
///     .expect("optional descriptor");
/// let Err(error) = optional.project_ref(ReflectedRef::new_str("text")) else {
///     panic!("a string is not Option<u32>");
/// };
/// assert!(matches!(error, OptionalProjectionError::TypeMismatch(_)));
/// ```
#[must_use]
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum OptionalProjectionError {
    /// This descriptor has no runtime projection function.
    #[error("optional value projection is unavailable")]
    Unavailable,
    /// The supplied erased value has a different concrete type.
    #[error(transparent)]
    TypeMismatch(#[from] TypeMismatch),
}
