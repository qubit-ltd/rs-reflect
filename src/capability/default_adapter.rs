// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Safe type-erased default constructors for concrete Rust types.

use std::any::Any;

/// A safe dynamic default constructor for one exact concrete Rust type.
///
/// # Examples
///
/// ```
/// use qubit_reflect::capability::DefaultAdapter;
///
/// let value = DefaultAdapter::new::<String>().create();
/// assert_eq!(value.downcast_ref::<String>().map(String::as_str), Some(""));
/// ```
#[derive(Clone, Copy)]
pub struct DefaultAdapter {
    create: fn() -> Box<dyn Any>,
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

    /// Creates an owned `T::default()` value behind `Any`.
    ///
    /// # Returns
    ///
    /// Returns an erased value containing the default value.
    ///
    /// # Panics
    ///
    /// Panics if the captured `Default` implementation panics.
    #[must_use]
    #[inline]
    pub fn create(&self) -> Box<dyn Any> {
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
/// Returns an erased value containing `T::default()`.
///
/// # Panics
///
/// Panics if `T::default()` panics.
fn create_default<T: Default + 'static>() -> Box<dyn Any> {
    Box::new(T::default())
}
