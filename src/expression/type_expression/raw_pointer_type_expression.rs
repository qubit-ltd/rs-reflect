// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Structural representations of Rust type expressions.

//! RawPointerTypeExpression structure and operations.

use crate::expression::DiagnosticText;
use crate::expression::TypeExpression;

/// A const or mutable raw pointer type.
///
/// # Examples
///
/// ```
/// use qubit_reflect::expression::RawPointerTypeExpression;
/// use qubit_reflect::expression::TypeExpression;
/// let pointer = RawPointerTypeExpression::new(false, TypeExpression::SelfType);
/// assert!(!pointer.is_mutable());
/// ```
#[derive(Clone, Debug)]
pub struct RawPointerTypeExpression {
    /// Whether this is a mutable raw pointer.
    pub(crate) mutable: bool,
    /// The pointee type.
    pub(crate) target: Box<TypeExpression>,
    /// Optional source-oriented diagnostic text excluded from identity.
    pub(crate) diagnostic: DiagnosticText,
}

impl RawPointerTypeExpression {
    /// Creates a raw pointer expression.
    ///
    /// # Parameters
    ///
    /// - `mutable`: Whether the pointer is mutable.
    /// - `target`: Pointee type expression.
    ///
    /// # Returns
    ///
    /// Returns the raw pointer expression with empty diagnostic text.
    #[must_use]
    pub fn new(mutable: bool, target: TypeExpression) -> Self {
        Self {
            mutable,
            target: Box::new(target),
            diagnostic: DiagnosticText::default(),
        }
    }
    /// Returns whether the pointer is mutable.
    ///
    /// # Returns
    ///
    /// Returns `true` for `*mut` and `false` for `*const`.
    #[must_use]
    #[inline]
    pub fn is_mutable(&self) -> bool {
        self.mutable
    }
    /// Returns the pointee type.
    ///
    /// # Returns
    ///
    /// Returns the pointed-to type expression.
    #[must_use]
    #[inline]
    pub fn target(&self) -> &TypeExpression {
        &self.target
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
    /// - `value`: Source-oriented pointer spelling used for diagnostics.
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

impl_identity_without_diagnostic!(RawPointerTypeExpression { mutable, target });
