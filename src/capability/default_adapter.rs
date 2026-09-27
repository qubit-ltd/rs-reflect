// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Safe dynamic default constructors for concrete Rust types.

use crate::value::DynamicOwned;
use crate::value::Local;

/// A safe dynamic default constructor for one exact concrete Rust type.
///
/// # Examples
///
/// ```
/// use qubit_reflect::capability::DefaultAdapter;
/// use qubit_reflect::value::DynamicOwned;
/// use qubit_reflect::value::Local;
///
/// let value: DynamicOwned<Local> = DefaultAdapter::new::<String>().create();
/// assert_eq!(value.downcast_ref::<String>().map(String::as_str), Some(""));
/// ```
#[derive(Clone, Copy)]
pub struct DefaultAdapter {
    create: fn() -> DynamicOwned<Local>,
}

impl DefaultAdapter {
    /// Creates an adapter after statically proving that `T` implements
    /// `Default`.
    ///
    /// # Type Parameters
    ///
    /// - `T`: The concrete type to construct dynamically.
    ///
    /// # Returns
    ///
    /// Returns an adapter bound to the exact type `T`.
    #[must_use]
    #[inline]
    pub fn new<T: Default + 'static>() -> Self {
        Self {
            create: create_default::<T>,
        }
    }

    /// Creates a local dynamic value containing `T::default()`.
    ///
    /// # Returns
    ///
    /// Returns a local dynamic value containing the default value.
    ///
    /// # Panics
    ///
    /// Panics if the captured `Default` implementation panics.
    #[must_use]
    #[inline]
    pub fn create(&self) -> DynamicOwned<Local> {
        (self.create)()
    }
}

/// Creates one default value of the statically registered concrete type.
///
/// # Type Parameters
///
/// - `T`: The concrete type whose `Default` implementation is invoked.
///
/// # Returns
///
/// Returns a local dynamic value containing `T::default()`.
///
/// # Panics
///
/// Panics if `T::default()` panics.
fn create_default<T: Default + 'static>() -> DynamicOwned<Local> {
    DynamicOwned::<Local>::new(T::default())
}
