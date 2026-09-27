// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Visibility origin of reflected struct and enum-variant fields.

use crate::identity::Visibility;

/// The source visibility fact recorded for a reflected field.
///
/// # Type Parameters
///
/// - `'a`: Lifetime of the borrowed declared visibility fact.
///
/// # Examples
///
/// ```
/// use qubit_reflect::access::FieldVisibility;
/// use qubit_reflect::identity::Visibility;
///
/// let visibility = Visibility::Public;
/// let field = FieldVisibility::Declared(&visibility);
/// assert_eq!(field.as_declared(), Some(&visibility));
/// assert!(!field.is_variant_inherited());
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FieldVisibility<'a> {
    /// A struct field's explicitly declared Rust visibility.
    Declared(&'a Visibility),
    /// An enum-variant field inherits the enum and variant access boundary.
    VariantInherited,
}

impl<'a> FieldVisibility<'a> {
    /// Returns the explicitly declared visibility of a struct field.
    ///
    /// `None` means this is an enum-variant field with inherited visibility.
    ///
    /// # Returns
    ///
    /// The declared visibility for a struct field, or `None` for an enum
    /// variant field.
    #[must_use]
    #[inline]
    pub const fn as_declared(self) -> Option<&'a Visibility> {
        match self {
            Self::Declared(visibility) => Some(visibility),
            Self::VariantInherited => None,
        }
    }

    /// Returns whether this field inherits an enum variant's access boundary.
    ///
    /// # Returns
    ///
    /// `true` for enum-variant fields and `false` for struct fields.
    #[must_use]
    #[inline]
    pub const fn is_variant_inherited(self) -> bool {
        matches!(self, Self::VariantInherited)
    }
}
