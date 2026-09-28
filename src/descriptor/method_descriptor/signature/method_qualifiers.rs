// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! method qualifiers definitions.

use crate::expression::FunctionAbi;

/// Qualifiers that affect whether a declaration can have an invocation adapter.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::MethodQualifiers;
/// let qualifiers = MethodQualifiers::new(false, false, false, None, false);
/// assert!(!qualifiers.is_async());
/// ```
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct MethodQualifiers {
    /// Whether the declaration is `async`.
    pub(crate) is_async: bool,
    /// Whether the declaration is `unsafe`.
    pub(crate) is_unsafe: bool,
    /// Whether the declaration is `const`.
    pub(crate) is_const: bool,
    /// The explicitly declared ABI, or `None` for the ordinary Rust ABI.
    pub(crate) abi: Option<FunctionAbi>,
    /// Whether the declaration has a variadic tail.
    pub(crate) is_variadic: bool,
}

impl MethodQualifiers {
    /// Creates the complete set of method qualifiers.
    ///
    /// # Parameters
    ///
    /// - `is_async`: Whether the declaration is asynchronous.
    /// - `is_unsafe`: Whether the declaration is unsafe.
    /// - `is_const`: Whether the declaration is const.
    /// - `abi`: Explicit ABI, or `None` for the Rust ABI.
    /// - `is_variadic`: Whether the declaration has a variadic tail.
    ///
    /// # Returns
    ///
    /// Returns the supplied method qualifiers.
    #[must_use]
    pub const fn new(
        is_async: bool,
        is_unsafe: bool,
        is_const: bool,
        abi: Option<FunctionAbi>,
        is_variadic: bool,
    ) -> Self {
        Self {
            is_async,
            is_unsafe,
            is_const,
            abi,
            is_variadic,
        }
    }

    /// Returns whether the declaration is asynchronous.
    ///
    /// # Returns
    ///
    /// Returns `true` for an `async` method.
    #[must_use]
    #[inline]
    pub const fn is_async(&self) -> bool {
        self.is_async
    }

    /// Returns whether the declaration is unsafe.
    ///
    /// # Returns
    ///
    /// Returns `true` for an `unsafe` method.
    #[must_use]
    #[inline]
    pub const fn is_unsafe(&self) -> bool {
        self.is_unsafe
    }

    /// Returns whether the declaration is const.
    ///
    /// # Returns
    ///
    /// Returns `true` for a `const` method.
    #[must_use]
    #[inline]
    pub const fn is_const(&self) -> bool {
        self.is_const
    }

    /// Returns the explicitly declared ABI.
    ///
    /// # Returns
    ///
    /// Returns the ABI, or `None` for the ordinary Rust ABI.
    #[must_use]
    #[inline]
    pub const fn abi(&self) -> Option<&FunctionAbi> {
        self.abi.as_ref()
    }

    /// Returns whether the declaration has a variadic tail.
    ///
    /// # Returns
    ///
    /// Returns `true` when the method has a variadic tail.
    #[must_use]
    #[inline]
    pub const fn is_variadic(&self) -> bool {
        self.is_variadic
    }
}

impl Default for MethodQualifiers {
    /// Returns the qualifiers of an ordinary safe Rust method.
    ///
    /// # Returns
    ///
    /// Returns default qualifiers with no async, unsafe, const, ABI, or
    /// variadic modifier.
    fn default() -> Self {
        Self::new(false, false, false, None, false)
    }
}
