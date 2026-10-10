// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! ImplAssociatedConstDescriptor metadata and behavior.

use crate::expression::TypeExpression;

/// One associated constant explicitly bound by an impl definition.
///
/// This descriptor is constructed by generated registration code.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::ImplAssociatedConstDescriptor;
/// use qubit_reflect::expression::ConcreteTypeExpression;
/// use qubit_reflect::expression::TypeExpression;
/// let binding = ImplAssociatedConstDescriptor::new(
///     "LIMIT",
///     TypeExpression::Concrete(ConcreteTypeExpression::new(["usize"], []).expect("non-empty path")),
/// );
/// assert_eq!(binding.rust_name(), "LIMIT");
/// ```
#[must_use]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImplAssociatedConstDescriptor {
    /// Name used in the Rust declaration.
    rust_name: &'static str,
    /// Declared type expression for the constant.
    declared_type: TypeExpression,
}

impl ImplAssociatedConstDescriptor {
    /// Creates declaration-level associated constant binding facts.
    ///
    /// # Parameters
    ///
    /// - `rust_name`: Name used by the associated constant declaration.
    /// - `declared_type`: Type expression declared for the constant.
    ///
    /// # Returns
    ///
    /// Returns the declaration-level binding facts.
    #[doc(hidden)]
    #[must_use = "the associated constant declaration facts are required by generated registration"]
    pub const fn new(rust_name: &'static str, declared_type: TypeExpression) -> Self {
        Self {
            rust_name,
            declared_type,
        }
    }

    /// Returns the Rust associated constant name.
    ///
    /// # Returns
    ///
    /// Returns the associated constant's Rust name.
    #[must_use]
    #[inline]
    pub const fn rust_name(&self) -> &'static str {
        self.rust_name
    }

    /// Returns the declared constant type.
    ///
    /// # Returns
    ///
    /// Returns the declared type expression.
    #[must_use]
    #[inline]
    pub const fn declared_type(&self) -> &TypeExpression {
        &self.declared_type
    }
}
