// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Structural representations of Rust type expressions.

//! FunctionPointerExpression structure and operations.

use crate::expression::DiagnosticText;
use crate::expression::FunctionAbi;
use crate::expression::FunctionSafety;
use crate::expression::LifetimeExpression;
use crate::expression::TypeExpression;

/// A function pointer signature.
///
/// # Examples
///
/// ```
/// use qubit_reflect::expression::FunctionAbi;
/// use qubit_reflect::expression::FunctionPointerExpression;
/// use qubit_reflect::expression::FunctionSafety;
/// use qubit_reflect::expression::TypeExpression;
/// let signature = FunctionPointerExpression::new(
///     FunctionAbi::Rust,
///     FunctionSafety::Safe,
///     false,
///     [],
///     [],
///     TypeExpression::Never,
/// );
/// assert!(signature.parameters().is_empty());
/// ```
#[derive(Clone, Debug)]
pub struct FunctionPointerExpression {
    /// The function's ABI.
    pub(crate) abi: FunctionAbi,
    /// The function's safety qualifier.
    pub(crate) safety: FunctionSafety,
    /// Whether the final argument is variadic.
    pub(crate) variadic: bool,
    /// Lifetimes introduced by a higher-ranked function pointer.
    pub(crate) higher_ranked_lifetimes: Box<[LifetimeExpression]>,
    /// Parameter types in declaration order.
    pub(crate) parameters: Box<[TypeExpression]>,
    /// The function return type.
    pub(crate) return_type: Box<TypeExpression>,
    /// Optional source-oriented diagnostic text excluded from identity.
    pub(crate) diagnostic: DiagnosticText,
}

impl FunctionPointerExpression {
    /// Creates a function pointer expression.
    ///
    /// # Parameters
    ///
    /// - `abi`: Calling convention.
    /// - `safety`: Safe or unsafe function pointer qualifier.
    /// - `variadic`: Whether the final parameter is variadic.
    /// - `higher_ranked_lifetimes`: Lifetimes introduced by a `for<...>`
    ///   binder.
    /// - `parameters`: Function parameter types in declaration order.
    /// - `return_type`: Function return type.
    ///
    /// # Returns
    ///
    /// Returns the function pointer expression with empty diagnostic text.
    pub fn new(
        abi: FunctionAbi,
        safety: FunctionSafety,
        variadic: bool,
        higher_ranked_lifetimes: impl Into<Box<[LifetimeExpression]>>,
        parameters: impl Into<Box<[TypeExpression]>>,
        return_type: TypeExpression,
    ) -> Self {
        Self {
            abi,
            safety,
            variadic,
            higher_ranked_lifetimes: higher_ranked_lifetimes.into(),
            parameters: parameters.into(),
            return_type: Box::new(return_type),
            diagnostic: DiagnosticText::default(),
        }
    }
    /// Returns the calling convention.
    ///
    /// # Returns
    ///
    /// Returns the function ABI.
    #[must_use]
    #[inline]
    pub fn abi(&self) -> &FunctionAbi {
        &self.abi
    }
    /// Returns the safety qualifier.
    ///
    /// # Returns
    ///
    /// Returns whether the function pointer is safe or unsafe.
    #[must_use]
    #[inline]
    pub fn safety(&self) -> &FunctionSafety {
        &self.safety
    }
    /// Returns whether the signature is variadic.
    ///
    /// # Returns
    ///
    /// Returns `true` when the final parameter is variadic.
    #[must_use]
    #[inline]
    pub fn is_variadic(&self) -> bool {
        self.variadic
    }
    /// Returns higher-ranked lifetimes.
    ///
    /// # Returns
    ///
    /// Returns lifetimes declared by the function pointer's binder.
    #[must_use]
    #[inline]
    pub fn higher_ranked_lifetimes(&self) -> &[LifetimeExpression] {
        &self.higher_ranked_lifetimes
    }
    /// Returns parameter types.
    ///
    /// # Returns
    ///
    /// Returns function parameter types in declaration order.
    #[must_use]
    #[inline]
    pub fn parameters(&self) -> &[TypeExpression] {
        &self.parameters
    }
    /// Returns the return type.
    ///
    /// # Returns
    ///
    /// Returns the function result type expression.
    #[must_use]
    #[inline]
    pub fn return_type(&self) -> &TypeExpression {
        &self.return_type
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
    /// - `value`: Source-oriented signature used for diagnostics.
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

impl_identity_without_diagnostic!(FunctionPointerExpression {
    abi,
    safety,
    variadic,
    higher_ranked_lifetimes,
    parameters,
    return_type
});
