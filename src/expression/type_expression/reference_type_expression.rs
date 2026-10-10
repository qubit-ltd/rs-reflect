// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Structural representations of Rust type expressions.

//! ReferenceTypeExpression structure and operations.

use crate::expression::DiagnosticText;
use crate::expression::LifetimeExpression;
use crate::expression::TypeExpression;

/// A shared or mutable reference type.
///
/// # Examples
///
/// ```
/// use qubit_reflect::expression::LifetimeExpression;
/// use qubit_reflect::expression::ReferenceTypeExpression;
/// use qubit_reflect::expression::TypeExpression;
/// let reference = ReferenceTypeExpression::new(
///     LifetimeExpression::Elided,
///     true,
///     TypeExpression::SelfType,
/// );
/// assert!(reference.is_mutable());
/// ```
#[derive(Clone, Debug)]
pub struct ReferenceTypeExpression {
    /// The reference lifetime, including [`LifetimeExpression::Elided`] when
    /// omitted.
    pub(crate) lifetime: LifetimeExpression,
    /// Whether this is a mutable reference.
    pub(crate) mutable: bool,
    /// The referenced type.
    pub(crate) target: Box<TypeExpression>,
    /// Optional source-oriented diagnostic text excluded from identity.
    pub(crate) diagnostic: DiagnosticText,
}

impl ReferenceTypeExpression {
    /// Creates a reference expression.
    ///
    /// # Parameters
    ///
    /// - `lifetime`: Source lifetime or elision category.
    /// - `mutable`: Whether the reference is mutable.
    /// - `target`: Referenced type.
    ///
    /// # Returns
    ///
    /// Returns the reference expression with empty diagnostic text.
    #[must_use]
    #[inline]
    pub fn new(lifetime: LifetimeExpression, mutable: bool, target: TypeExpression) -> Self {
        Self {
            lifetime,
            mutable,
            target: Box::new(target),
            diagnostic: DiagnosticText::default(),
        }
    }
    /// Returns the reference lifetime.
    ///
    /// # Returns
    ///
    /// Returns the named, static, elided, or placeholder lifetime.
    #[must_use]
    #[inline]
    pub fn lifetime(&self) -> &LifetimeExpression {
        &self.lifetime
    }
    /// Returns whether the reference is mutable.
    ///
    /// # Returns
    ///
    /// Returns `true` for `&mut` and `false` for a shared reference.
    #[must_use]
    #[inline]
    pub fn is_mutable(&self) -> bool {
        self.mutable
    }
    /// Returns the referenced type.
    ///
    /// # Returns
    ///
    /// Returns the target type expression.
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
    #[inline]
    pub fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }
    /// Attaches diagnostic text.
    ///
    /// # Parameters
    ///
    /// - `value`: Source-oriented reference spelling used for diagnostics.
    ///
    /// # Returns
    ///
    /// Returns the expression with diagnostic text attached; structural
    /// identity is unchanged.
    #[must_use]
    #[inline]
    pub fn with_diagnostic(mut self, value: impl Into<Box<str>>) -> Self {
        self.diagnostic = DiagnosticText::from(value.into());
        self
    }
}

impl_identity_without_diagnostic!(ReferenceTypeExpression {
    lifetime,
    mutable,
    target
});
