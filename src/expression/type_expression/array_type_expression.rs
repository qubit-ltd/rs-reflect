// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Structural representations of Rust type expressions.

use crate::expression::ConstExpression;
use crate::expression::DiagnosticText;
use crate::expression::TypeExpression;

/// An array type and its structural length expression.
///
/// # Examples
///
/// ```
/// use qubit_reflect::expression::ArrayTypeExpression;
/// use qubit_reflect::expression::ConstExpression;
/// use qubit_reflect::expression::TypeExpression;
/// let array = ArrayTypeExpression::new(
///     TypeExpression::SelfType,
///     ConstExpression::UnsignedInteger(4),
/// );
/// assert_eq!(array.length(), &ConstExpression::UnsignedInteger(4));
/// ```
#[derive(Clone, Debug)]
pub struct ArrayTypeExpression {
    /// The repeated element type.
    pub(crate) element: Box<TypeExpression>,
    /// The array length expression.
    pub(crate) length: ConstExpression,
    /// Optional source-oriented diagnostic text excluded from identity.
    pub(crate) diagnostic: DiagnosticText,
}

impl ArrayTypeExpression {
    /// Creates an array expression.
    ///
    /// # Parameters
    ///
    /// - `element`: Type repeated in each array element.
    /// - `length`: Structural const expression for the array length.
    ///
    /// # Returns
    ///
    /// Returns the array expression with empty diagnostic text.
    #[must_use]
    pub fn new(element: TypeExpression, length: ConstExpression) -> Self {
        Self {
            element: Box::new(element),
            length,
            diagnostic: DiagnosticText::default(),
        }
    }
    /// Returns the element type.
    ///
    /// # Returns
    ///
    /// Returns the repeated element type.
    #[must_use]
    #[inline]
    pub fn element(&self) -> &TypeExpression {
        &self.element
    }
    /// Returns the structural length expression.
    ///
    /// # Returns
    ///
    /// Returns the array length expression.
    #[must_use]
    #[inline]
    pub fn length(&self) -> &ConstExpression {
        &self.length
    }
    /// Returns diagnostic text when present.
    ///
    /// # Returns
    ///
    /// Returns the source-oriented spelling, or `None` when absent.
    #[must_use]
    #[inline]
    pub fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }
    /// Attaches diagnostic text.
    ///
    /// # Parameters
    ///
    /// - `value`: Source-oriented array spelling used for diagnostics.
    ///
    /// # Returns
    ///
    /// Returns the expression with diagnostic text attached; structural
    /// identity is unchanged.
    #[must_use]
    pub fn with_diagnostic(mut self, value: impl Into<Box<str>>) -> Self {
        self.diagnostic = DiagnosticText::from(value.into());
        self
    }
}

impl_identity_without_diagnostic!(ArrayTypeExpression { element, length });
