// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Safe dynamic clone operations for concrete Rust types.

use std::any::Any;
use std::any::TypeId;

use crate::error::TypeMismatch;
use crate::value::DynamicOwned;
use crate::value::Local;

/// A safe dynamic clone operation for one exact concrete Rust type.
///
/// # Examples
///
/// ```
/// use qubit_reflect::capability::CloneAdapter;
/// use qubit_reflect::value::DynamicOwned;
/// use qubit_reflect::value::Local;
///
/// let adapter = CloneAdapter::new::<String>();
/// let value = DynamicOwned::<Local>::new(String::from("value"));
/// let cloned = adapter.clone_owned(&value).expect("matching type");
/// assert_eq!(cloned.downcast_ref::<String>().map(String::as_str), Some("value"));
/// ```
#[derive(Clone, Copy)]
pub struct CloneAdapter {
    /// Stores the monomorphized clone operation for the registered type.
    clone_owned: fn(&DynamicOwned<Local>) -> Result<DynamicOwned<Local>, TypeMismatch>,
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
            clone_owned: clone_owned::<T>,
        }
    }

    /// Clones a local dynamic value when it contains the registered exact type.
    ///
    /// Returns [`TypeMismatch`] without changing `value` when its concrete type
    /// differs from the type captured by this adapter.
    ///
    /// # Parameters
    ///
    /// - `value`: The dynamic value to clone.
    ///
    /// # Returns
    ///
    /// Returns an owned clone of the exact registered type.
    ///
    /// # Errors
    ///
    /// Returns [`TypeMismatch`] when `value` does not contain the adapter's
    /// registered type.
    ///
    /// # Panics
    ///
    /// Panics if the captured `Clone` implementation panics or if local dynamic
    /// storage violates its `Any` compatibility invariant.
    #[inline]
    pub fn clone_owned(&self, value: &DynamicOwned<Local>) -> Result<DynamicOwned<Local>, TypeMismatch> {
        (self.clone_owned)(value)
    }
}

/// Clones one exact dynamic concrete type after checking its runtime identity.
///
/// # Type Parameters
///
/// - `T`: The concrete type captured by the adapter.
///
/// # Parameters
///
/// - `value`: The dynamic value to clone.
///
/// # Returns
///
/// Returns an owned clone of the value.
///
/// # Errors
///
/// Returns [`TypeMismatch`] if the dynamic value does not contain `T`.
///
/// # Panics
///
/// Panics only if a local dynamic value violates its invariant that storage is
/// compatible with `Any`.
fn clone_owned<T: Clone + 'static>(value: &DynamicOwned<Local>) -> Result<DynamicOwned<Local>, TypeMismatch> {
    let Some(value) = value.downcast_ref::<T>() else {
        let actual = value
            .as_any()
            .map(Any::type_id)
            .expect("DynamicOwned<Local> always contains Any-compatible storage");
        return Err(TypeMismatch::new(TypeId::of::<T>(), actual));
    };
    Ok(DynamicOwned::<Local>::new(value.clone()))
}
