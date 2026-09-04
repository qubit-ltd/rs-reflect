// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

//! Source-level field facts for reflected generic declarations.

use crate::expression::TypeExpression;
use crate::identity::Visibility;

/// The immutable source-level description of one field.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldDefinitionDescriptor {
    index: usize,
    rust_name: Option<&'static str>,
    query_name: Option<&'static str>,
    ty: TypeExpression,
    visibility: Visibility,
}

impl FieldDefinitionDescriptor {
    /// Creates one declaration field without runtime access operations.
    #[doc(hidden)]
    #[must_use]
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
    #[must_use]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Returns the Rust field name, or `None` for positional fields.
    #[must_use]
    pub const fn rust_name(&self) -> Option<&'static str> {
        self.rust_name
    }

    /// Returns the lookup name, or `None` for positional fields.
    #[must_use]
    pub const fn query_name(&self) -> Option<&'static str> {
        self.query_name
    }

    /// Returns the source-level field type expression.
    #[must_use]
    pub const fn ty(&self) -> &TypeExpression {
        &self.ty
    }

    /// Returns the normalized source visibility.
    #[must_use]
    pub const fn visibility(&self) -> &Visibility {
        &self.visibility
    }
}
