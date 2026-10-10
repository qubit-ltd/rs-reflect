// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Structural representations of Rust type expressions.

//! AssociatedTypeExpression structure and operations.

use crate::expression::DiagnosticText;
use crate::expression::ExpressionName;
use crate::expression::GenericArgument;
use crate::expression::TypeExpression;

/// An associated type projection.
///
/// # Examples
///
/// ```
/// use qubit_reflect::expression::AssociatedTypeExpression;
/// use qubit_reflect::expression::TypeExpression;
/// let projection = AssociatedTypeExpression::new(
///     TypeExpression::parameter("T").expect("valid parameter"),
///     None,
///     "Item",
///     [],
/// );
/// assert_eq!(projection.item(), "Item");
/// ```
#[derive(Clone, Debug)]
pub struct AssociatedTypeExpression {
    /// The self type whose associated item is projected.
    pub(crate) self_type: Box<TypeExpression>,
    /// The optional qualifying trait path from an `as Trait` clause.
    pub(crate) trait_path: Option<Box<TypeExpression>>,
    /// The associated item name.
    pub(crate) item: ExpressionName,
    /// Generic arguments applied to the associated type.
    pub(crate) arguments: Box<[GenericArgument]>,
    /// Optional source-oriented diagnostic text excluded from identity.
    pub(crate) diagnostic: DiagnosticText,
}

impl AssociatedTypeExpression {
    /// Creates an associated type projection.
    ///
    /// # Parameters
    ///
    /// - `self_type`: Type on which the associated type is projected.
    /// - `trait_path`: Optional trait qualification after `as`.
    /// - `item`: Associated type name.
    /// - `arguments`: Generic arguments applied to the associated type.
    ///
    /// # Returns
    ///
    /// Returns the projection with empty diagnostic text.
    #[must_use]
    pub fn new(
        self_type: TypeExpression,
        trait_path: Option<TypeExpression>,
        item: impl Into<ExpressionName>,
        arguments: impl Into<Box<[GenericArgument]>>,
    ) -> Self {
        Self {
            self_type: Box::new(self_type),
            trait_path: trait_path.map(Box::new),
            item: item.into(),
            arguments: arguments.into(),
            diagnostic: DiagnosticText::default(),
        }
    }

    /// Returns the projected self type.
    ///
    /// # Returns
    ///
    /// Returns the type on which the associated type is projected.
    #[must_use]
    #[inline]
    pub fn self_type(&self) -> &TypeExpression {
        &self.self_type
    }

    /// Returns the optional qualifying trait path.
    ///
    /// # Returns
    ///
    /// Returns the trait qualification, or `None` for an unqualified
    /// projection.
    #[must_use]
    #[inline]
    pub fn trait_path(&self) -> Option<&TypeExpression> {
        self.trait_path.as_deref()
    }

    /// Returns the associated item name.
    ///
    /// # Returns
    ///
    /// Returns the associated type identifier.
    #[must_use]
    #[inline]
    pub fn item(&self) -> &str {
        self.item.as_str()
    }

    /// Returns associated type arguments.
    ///
    /// # Returns
    ///
    /// Returns generic arguments applied to the projection.
    #[must_use]
    #[inline]
    pub fn arguments(&self) -> &[GenericArgument] {
        &self.arguments
    }

    /// Returns source-oriented diagnostic text when present.
    ///
    /// # Returns
    ///
    /// Returns diagnostic text, or `None` when absent.
    #[must_use]
    #[inline]
    pub fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }

    /// Attaches source-oriented diagnostic text.
    ///
    /// # Parameters
    ///
    /// - `value`: Source-oriented projection spelling used for diagnostics.
    ///
    /// # Returns
    ///
    /// Returns the projection with diagnostic text attached; structural
    /// identity is unchanged.
    #[must_use]
    pub fn with_diagnostic(mut self, value: impl Into<Box<str>>) -> Self {
        self.diagnostic = DiagnosticText::from(value.into());
        self
    }
}

impl_identity_without_diagnostic!(AssociatedTypeExpression {
    self_type,
    trait_path,
    item,
    arguments
});
