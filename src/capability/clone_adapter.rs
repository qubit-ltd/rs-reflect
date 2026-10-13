// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Safe type-erased clone operations for concrete Rust types.

use std::any::Any;
use std::any::TypeId;

use crate::error::TypeMismatch;

type CloneOperation = fn(&dyn Any) -> Result<Box<dyn Any>, TypeMismatch>;

/// A safe dynamic clone operation for one exact concrete Rust type.
///
/// # Examples
///
/// ```
/// use qubit_reflect::capability::CloneAdapter;
///
/// let adapter = CloneAdapter::new::<String>();
/// let value = String::from("value");
/// let cloned = adapter.clone_value(&value).expect("matching type");
/// assert_eq!(cloned.downcast_ref::<String>().map(String::as_str), Some("value"));
/// ```
#[derive(Clone, Copy)]
pub struct CloneAdapter {
    /// Stores the monomorphized clone operation for the registered type.
    clone_value: CloneOperation,
}

impl CloneAdapter {
    /// Creates an adapter after statically proving that `T` implements `Clone`.
    ///
    /// # Type Parameters
    ///
    /// - `T`: The concrete type to clone dynamically.
    ///
    /// # Returns
    ///
    /// Returns an adapter bound to the exact type `T`.
    #[must_use]
    #[inline]
    pub fn new<T: Clone + 'static>() -> Self {
        Self {
            clone_value: clone_value::<T>,
        }
    }

    /// Clones an ordinary value when it has the registered exact type.
    ///
    /// Returns [`TypeMismatch`] without changing `value` when its concrete type
    /// differs from the type captured by this adapter.
    ///
    /// # Parameters
    ///
    /// - `value`: The erased value to clone.
    ///
    /// # Returns
    ///
    /// Returns a boxed clone of the exact registered type.
    ///
    /// # Errors
    ///
    /// Returns [`TypeMismatch`] when `value` does not contain the adapter's
    /// registered type.
    ///
    /// # Panics
    ///
    /// Panics if the captured `Clone` implementation panics.
    #[inline]
    pub fn clone_value(&self, value: &dyn Any) -> Result<Box<dyn Any>, TypeMismatch> {
        (self.clone_value)(value)
    }
}

/// Clones one exact concrete type after checking its runtime identity.
///
/// # Type Parameters
///
/// - `T`: The concrete type captured by the adapter.
///
/// # Parameters
///
/// - `value`: The erased value to clone.
///
/// # Returns
///
/// Returns a boxed clone of the value.
///
/// # Errors
///
/// Returns [`TypeMismatch`] if the dynamic value does not contain `T`.
///
/// # Panics
///
/// Panics only if the captured `Clone` implementation panics.
fn clone_value<T: Clone + 'static>(value: &dyn Any) -> Result<Box<dyn Any>, TypeMismatch> {
    let Some(value) = value.downcast_ref::<T>() else {
        return Err(TypeMismatch::new(TypeId::of::<T>(), value.type_id()));
    };
    Ok(Box::new(value.clone()))
}
