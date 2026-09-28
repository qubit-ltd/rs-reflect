// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Exact invocation argument type and mode expectations.

use std::any::TypeId;

use crate::invoke::InvocationInputMode;

/// The exact type and passing mode expected for one positional argument.
///
/// # Examples
///
/// ```
/// use qubit_reflect::invoke::{ArgumentExpectation, InvocationInputMode};
/// let expected = ArgumentExpectation::owned::<u32>();
/// assert_eq!(expected.mode(), InvocationInputMode::Owned);
/// assert_eq!(expected.type_name(), "u32");
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArgumentExpectation {
    /// Required ownership or borrowing mode.
    mode: InvocationInputMode,
    /// Exact process-local type identity required by the adapter.
    type_id: TypeId,
    /// Rust type name retained for diagnostics.
    type_name: &'static str,
}

impl ArgumentExpectation {
    /// Creates an expectation for an owned `T` argument.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Exact owned argument type.
    ///
    /// # Returns
    ///
    /// Returns an owned argument expectation for `T`.
    #[must_use]
    pub fn owned<T: ?Sized + 'static>() -> Self {
        Self::new::<T>(InvocationInputMode::Owned)
    }

    /// Creates an expectation for a shared `T` argument.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Exact shared-borrow argument type.
    ///
    /// # Returns
    ///
    /// Returns a shared-borrow expectation for `T`.
    #[must_use]
    pub fn borrowed<T: ?Sized + 'static>() -> Self {
        Self::new::<T>(InvocationInputMode::Ref)
    }

    /// Creates an expectation for a mutable `T` argument.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Exact mutable-borrow argument type.
    ///
    /// # Returns
    ///
    /// Returns a mutable-borrow expectation for `T`.
    #[must_use]
    pub fn borrowed_mut<T: ?Sized + 'static>() -> Self {
        Self::new::<T>(InvocationInputMode::Mut)
    }

    /// Returns the required argument mode.
    ///
    /// # Returns
    ///
    /// Returns the required ownership or borrowing mode.
    #[must_use]
    #[inline]
    pub const fn mode(self) -> InvocationInputMode {
        self.mode
    }

    /// Returns the exact expected process-local Rust type identity.
    ///
    /// # Returns
    ///
    /// Returns the exact expected `TypeId`.
    #[must_use]
    #[inline]
    pub const fn type_id(self) -> TypeId {
        self.type_id
    }

    /// Returns the expected Rust type name for diagnostics.
    ///
    /// # Returns
    ///
    /// Returns the compiler-provided type name of the expected value.
    #[must_use]
    #[inline]
    pub const fn type_name(self) -> &'static str {
        self.type_name
    }

    /// Creates one exact argument expectation for `T`.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Exact argument type.
    ///
    /// # Parameters
    ///
    /// - `mode`: Required ownership or borrowing mode.
    ///
    /// # Returns
    ///
    /// Returns an expectation for `T` with the supplied mode.
    fn new<T: ?Sized + 'static>(mode: InvocationInputMode) -> Self {
        Self {
            mode,
            type_id: TypeId::of::<T>(),
            type_name: std::any::type_name::<T>(),
        }
    }
}
