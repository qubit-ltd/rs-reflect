// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::descriptor::TextKind;

/// The typed view of an owned or borrowed text descriptor.
///
/// # Examples
///
/// ```
/// use qubit_reflect::TypeDescriptor;
/// let text = TypeDescriptor::of::<String>().as_text().expect("text type");
/// assert_eq!(text.kind(), qubit_reflect::descriptor::TextKind::String);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct TextTypeDescriptor {
    /// Owned or borrowed UTF-8 text representation.
    kind: TextKind,
}

impl TextTypeDescriptor {
    /// Creates a text view for internal descriptor construction.
    ///
    /// # Parameters
    ///
    /// - `kind`: Owned or borrowed UTF-8 text representation.
    ///
    /// # Returns
    ///
    /// Returns the typed view for `kind`.
    pub(crate) const fn new(kind: TextKind) -> Self {
        Self { kind }
    }

    /// Returns the exact text representation.
    ///
    /// # Returns
    ///
    /// Returns the owned or borrowed text category.
    #[must_use]
    #[inline]
    pub const fn kind(&self) -> TextKind {
        self.kind
    }
}
