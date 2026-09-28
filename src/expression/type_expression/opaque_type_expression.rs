// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Structural representations of Rust type expressions.

//! OpaqueTypeExpression structure and operations.

use crate::expression::DiagnosticText;
use crate::expression::PredicateDescriptor;

/// An `impl Trait` opaque type and the predicates it must satisfy.
///
/// # Examples
///
/// ```
/// use qubit_reflect::expression::OpaqueTypeExpression;
/// let opaque = OpaqueTypeExpression::new([]);
/// assert!(opaque.bounds().is_empty());
/// ```
#[derive(Clone, Debug)]
pub struct OpaqueTypeExpression {
    /// Trait and lifetime predicates in declaration order.
    pub(crate) bounds: Box<[PredicateDescriptor]>,
    /// Optional source-oriented diagnostic text excluded from identity.
    pub(crate) diagnostic: DiagnosticText,
}

impl OpaqueTypeExpression {
    /// Creates an opaque type expression.
    ///
    /// # Parameters
    ///
    /// - `bounds`: Trait and lifetime predicates constraining the opaque type.
    ///
    /// # Returns
    ///
    /// Returns an opaque type expression with empty diagnostic text.
    pub fn new(bounds: impl Into<Box<[PredicateDescriptor]>>) -> Self {
        Self {
            bounds: bounds.into(),
            diagnostic: DiagnosticText::default(),
        }
    }
    /// Returns opaque bounds.
    ///
    /// # Returns
    ///
    /// Returns trait and lifetime predicates in declaration order.
    #[must_use]
    pub fn bounds(&self) -> &[PredicateDescriptor] {
        &self.bounds
    }
    /// Returns diagnostic text when present.
    ///
    /// # Returns
    ///
    /// Returns the source-oriented spelling, or `None` when absent.
    #[must_use]
    pub fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }
    /// Attaches diagnostic text.
    ///
    /// # Parameters
    ///
    /// - `value`: Source-oriented opaque type spelling used for diagnostics.
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

impl_identity_without_diagnostic!(OpaqueTypeExpression { bounds });
