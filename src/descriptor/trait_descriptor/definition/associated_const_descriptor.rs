// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Declaration facts for an associated constant.

use crate::descriptor::trait_descriptor::TraitApplicationSubstitutions;
use crate::expression::TypeExpression;

/// One associated constant declaration.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::AssociatedConstDescriptor;
/// use qubit_reflect::expression::{ConcreteTypeExpression, TypeExpression};
///
/// let item = AssociatedConstDescriptor::new(
///     0,
///     "LIMIT",
///     "limit",
///     TypeExpression::Concrete(
///         ConcreteTypeExpression::new(["usize"], []).expect("non-empty path"),
///     ),
///     false,
/// );
/// assert_eq!(item.query_name(), "limit");
/// ```
#[derive(Clone, Debug)]
pub struct AssociatedConstDescriptor {
    /// Zero-based source position among associated constants.
    index: usize,
    /// Rust declaration identifier.
    rust_name: &'static str,
    /// Reflection lookup name.
    query_name: &'static str,
    /// Declared structural type.
    declared_type: TypeExpression,
    /// Whether the trait declaration provides a default value.
    has_default: bool,
}

impl AssociatedConstDescriptor {
    /// Creates associated constant facts in declaration order.
    ///
    /// # Parameters
    ///
    /// - `index`: Zero-based source declaration position.
    /// - `rust_name`: Source Rust identifier.
    /// - `query_name`: Reflection lookup name.
    /// - `declared_type`: Structural type of the constant.
    /// - `has_default`: Whether the declaration supplies a value.
    ///
    /// # Returns
    ///
    /// Returns immutable associated-constant facts.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(
        index: usize,
        rust_name: &'static str,
        query_name: &'static str,
        declared_type: TypeExpression,
        has_default: bool,
    ) -> Self {
        Self {
            index,
            rust_name,
            query_name,
            declared_type,
            has_default,
        }
    }

    /// Returns the source declaration index.
    ///
    /// # Returns
    ///
    /// Returns the zero-based position among associated constants.
    #[must_use]
    #[inline]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Returns the Rust declaration name.
    ///
    /// # Returns
    ///
    /// Returns the source identifier.
    #[must_use]
    #[inline]
    pub const fn rust_name(&self) -> &'static str {
        self.rust_name
    }

    /// Returns the lookup name.
    ///
    /// # Returns
    ///
    /// Returns the reflection query name.
    #[must_use]
    #[inline]
    pub const fn query_name(&self) -> &'static str {
        self.query_name
    }

    /// Returns the declared constant type.
    ///
    /// # Returns
    ///
    /// Returns the structural declared type.
    #[must_use]
    #[inline]
    pub const fn declared_type(&self) -> &TypeExpression {
        &self.declared_type
    }

    /// Returns whether the trait declaration provides a default value.
    ///
    /// # Returns
    ///
    /// Returns `true` when the source trait provides a default.
    #[must_use]
    #[inline]
    pub const fn has_default(&self) -> bool {
        self.has_default
    }

    /// Applies one concrete trait application to this declaration.
    ///
    /// # Parameters
    ///
    /// - `substitutions`: Concrete generic substitutions.
    ///
    /// # Returns
    ///
    /// Returns this declaration with its type expression substituted.
    pub(in crate::descriptor::trait_descriptor) fn substituted(
        self,
        substitutions: &TraitApplicationSubstitutions,
    ) -> Self {
        Self {
            declared_type: substitutions.type_expression(&self.declared_type),
            ..self
        }
    }
}
