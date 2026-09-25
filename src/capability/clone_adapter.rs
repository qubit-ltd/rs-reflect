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
#[derive(Clone, Copy)]
pub struct CloneAdapter {
    clone_owned: fn(&DynamicOwned<Local>) -> Result<DynamicOwned<Local>, TypeMismatch>,
}

impl CloneAdapter {
    /// Creates an adapter after statically proving that `T` implements `Clone`.
    #[must_use]
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
    /// # Returns
    ///
    /// Returns an owned clone of the exact registered type.
    ///
    /// # Errors
    ///
    /// Returns [`TypeMismatch`] when `value` does not contain the adapter's
    /// registered type.
    pub fn clone_owned(&self, value: &DynamicOwned<Local>) -> Result<DynamicOwned<Local>, TypeMismatch> {
        (self.clone_owned)(value)
    }
}

/// Clones one exact dynamic concrete type after checking its runtime identity.
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
