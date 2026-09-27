// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Source-level field facts for reflected generic declarations.

use crate::expression::TypeExpression;
use crate::identity::Visibility;

/// The immutable source-level description of one field.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::FieldDefinitionDescriptor;
/// use qubit_reflect::expression::TypeExpression;
/// use qubit_reflect::identity::Visibility;
///
/// let field = FieldDefinitionDescriptor::new(
///     0,
///     Some("value"),
///     Some("value"),
///     TypeExpression::Parameter("T".into()),
///     Visibility::Public,
/// );
/// assert_eq!(field.query_name(), Some("value"));
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldDefinitionDescriptor {
    /// Zero-based field position in the source declaration.
    index: usize,
    /// Source Rust identifier, absent for tuple fields.
    rust_name: Option<&'static str>,
    /// Reflected lookup identifier, absent for tuple fields.
    query_name: Option<&'static str>,
    /// Structural Rust type expression retained from the declaration.
    ty: TypeExpression,
    /// Normalized source visibility.
    visibility: Visibility,
}

impl FieldDefinitionDescriptor {
    /// Creates one declaration field without runtime access operations.
    ///
    /// # Parameters
    ///
    /// - `index`: The zero-based source field position.
    /// - `rust_name`: The source identifier, or `None` for tuple fields.
    /// - `query_name`: The lookup identifier, or `None` for tuple fields.
    /// - `ty`: The source-level field type expression.
    /// - `visibility`: The normalized source visibility.
    ///
    /// # Returns
    ///
    /// Returns immutable source metadata for the field.
    #[doc(hidden)]
    #[must_use]
    #[inline]
    pub const fn new(
        index: usize,
        rust_name: Option<&'static str>,
        query_name: Option<&'static str>,
        ty: TypeExpression,
        visibility: Visibility,
    ) -> Self {
        Self {
            index,
            rust_name,
            query_name,
            ty,
            visibility,
        }
    }

    /// Returns the zero-based source declaration index.
    ///
    /// # Returns
    ///
    /// Returns the field's index in its declaring item.
    #[must_use]
    #[inline]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Returns the Rust field name, or `None` for positional fields.
    ///
    /// # Returns
    ///
    /// Returns the source identifier, or `None` for positional fields.
    #[must_use]
    #[inline]
    pub const fn rust_name(&self) -> Option<&'static str> {
        self.rust_name
    }

    /// Returns the lookup name, or `None` for positional fields.
    ///
    /// # Returns
    ///
    /// Returns the reflected lookup identifier, or `None` for positional
    /// fields.
    #[must_use]
    #[inline]
    pub const fn query_name(&self) -> Option<&'static str> {
        self.query_name
    }

    /// Returns the source-level field type expression.
    ///
    /// # Returns
    ///
    /// Returns the structural type expression retained from the declaration.
    #[must_use]
    #[inline]
    pub const fn ty(&self) -> &TypeExpression {
        &self.ty
    }

    /// Returns the normalized source visibility.
    ///
    /// # Returns
    ///
    /// Returns the source visibility category and any restricted path.
    #[must_use]
    #[inline]
    pub const fn visibility(&self) -> &Visibility {
        &self.visibility
    }
}
