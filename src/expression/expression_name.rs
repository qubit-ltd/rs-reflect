// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Validated names used by structural expressions.

use std::borrow::Borrow;

use crate::expression::ExpressionError;

/// A non-empty name occurring in a structural Rust expression.
///
/// The value deliberately validates only the cross-expression invariant that
/// names are non-empty. The macro parser remains responsible for applying the
/// more specific Rust grammar of a lifetime, parameter, associated item, or
/// ABI name at its source boundary.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ExpressionName(Box<str>);

impl ExpressionName {
    /// Creates a structural name after rejecting empty text.
    ///
    /// # Errors
    ///
    /// Returns [`ExpressionError::EmptyName`] when `value` is empty.
    pub fn new(value: impl Into<Box<str>>) -> Result<Self, ExpressionError> {
        let value = value.into();
        if value.is_empty() {
            return Err(ExpressionError::EmptyName);
        }
        Ok(Self(value))
    }

    /// Returns the validated name text.
    #[must_use]
    #[inline(always)]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for ExpressionName {
    #[inline(always)]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for ExpressionName {
    #[inline(always)]
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl From<&str> for ExpressionName {
    /// Creates a validated name from a string literal or borrowed string.
    ///
    /// # Panics
    ///
    /// Panics when `value` is empty. User-controlled text should use
    /// [`ExpressionName::new`] to receive a structured error instead.
    fn from(value: &str) -> Self {
        Self::new(value).expect("expression names converted with From must not be empty")
    }
}

impl From<String> for ExpressionName {
    /// Creates a validated name from owned text.
    ///
    /// # Panics
    ///
    /// Panics when `value` is empty. User-controlled text should use
    /// [`ExpressionName::new`] to receive a structured error instead.
    fn from(value: String) -> Self {
        Self::new(value).expect("expression names converted with From must not be empty")
    }
}

impl From<Box<str>> for ExpressionName {
    /// Creates a validated name from boxed text.
    ///
    /// # Panics
    ///
    /// Panics when `value` is empty. User-controlled text should use
    /// [`ExpressionName::new`] to receive a structured error instead.
    fn from(value: Box<str>) -> Self {
        Self::new(value).expect("expression names converted with From must not be empty")
    }
}
