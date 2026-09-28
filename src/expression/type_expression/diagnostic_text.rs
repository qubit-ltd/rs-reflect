// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Structural representations of Rust type expressions.

//! DiagnosticText structure and operations.

/// Source-oriented text that supplements diagnostics.
///
/// This value has ordinary text equality and hashing. Descriptor
/// implementations deliberately exclude diagnostic fields from their structural
/// identity.
///
/// # Examples
///
/// ```
/// use qubit_reflect::expression::DiagnosticText;
/// let text = DiagnosticText::from("source spelling");
/// assert_eq!(text.as_deref(), Some("source spelling"));
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct DiagnosticText(
    /// Optional source-oriented text retained for diagnostic output.
    pub(crate) Option<Box<str>>,
);

impl DiagnosticText {
    /// Returns the diagnostic text when present.
    ///
    /// # Returns
    ///
    /// Returns the retained text, or `None` when no diagnostic was attached.
    #[must_use]
    pub fn as_deref(&self) -> Option<&str> {
        self.0.as_deref()
    }
}

impl From<Box<str>> for DiagnosticText {
    /// Wraps owned diagnostic text.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned text retained for diagnostics.
    ///
    /// # Returns
    ///
    /// Returns diagnostic text containing `value`.
    fn from(value: Box<str>) -> Self {
        Self(Some(value))
    }
}

impl From<&str> for DiagnosticText {
    /// Copies borrowed text into diagnostic storage.
    ///
    /// # Parameters
    ///
    /// - `value`: Text retained for diagnostics.
    ///
    /// # Returns
    ///
    /// Returns diagnostic text containing a copy of `value`.
    fn from(value: &str) -> Self {
        Self(Some(value.into()))
    }
}

impl From<String> for DiagnosticText {
    /// Moves owned text into diagnostic storage.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned text retained for diagnostics.
    ///
    /// # Returns
    ///
    /// Returns diagnostic text containing `value`.
    fn from(value: String) -> Self {
        Self(Some(value.into_boxed_str()))
    }
}
