// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Structural representations of Rust lifetime syntax.

use crate::expression::ExpressionError;
use crate::expression::ExpressionName;

/// A lifetime that appears in a type, bound, or generic declaration.
///
/// Named lifetimes omit the leading apostrophe so they can be compared and
/// displayed without retaining parser tokens.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum LifetimeExpression {
    /// The distinguished `'static` lifetime.
    Static,
    /// A named lifetime such as `'a`, stored as `a`.
    Named(ExpressionName),
    /// An elided lifetime supplied by Rust's lifetime elision rules.
    Elided,
    /// The anonymous placeholder lifetime `'_`.
    Placeholder,
}

impl LifetimeExpression {
    /// Creates a named lifetime expression without the leading apostrophe.
    ///
    /// # Errors
    ///
    /// Returns [`ExpressionError::EmptyName`] when `name` is empty.
    pub fn named(name: impl Into<Box<str>>) -> Result<Self, ExpressionError> {
        ExpressionName::new(name).map(Self::Named)
    }
}
