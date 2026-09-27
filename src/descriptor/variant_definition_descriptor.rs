// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Source-level enum variant facts for reflected generic declarations.

use crate::descriptor::FieldDefinitionDescriptor;
use crate::descriptor::VariantKind;

/// The immutable source-level description of one enum variant.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::{VariantDefinitionDescriptor, VariantKind};
/// let variant = VariantDefinitionDescriptor::new(0, "Ready", "Ready", VariantKind::Unit, &[]);
/// assert_eq!(variant.kind(), VariantKind::Unit);
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VariantDefinitionDescriptor {
    index: usize,
    rust_name: &'static str,
    query_name: &'static str,
    kind: VariantKind,
    fields: &'static [FieldDefinitionDescriptor],
}

impl VariantDefinitionDescriptor {
    /// Creates one declaration variant without runtime operations.
    ///
    /// # Parameters
    ///
    /// - `index`: Zero-based source declaration position.
    /// - `rust_name`: Source Rust identifier.
    /// - `query_name`: Reflection lookup name.
    /// - `kind`: Unit, tuple, or struct-like source shape.
    /// - `fields`: Field metadata in declaration order.
    ///
    /// # Returns
    ///
    /// Returns immutable source metadata for the variant.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(
        index: usize,
        rust_name: &'static str,
        query_name: &'static str,
        kind: VariantKind,
        fields: &'static [FieldDefinitionDescriptor],
    ) -> Self {
        Self {
            index,
            rust_name,
            query_name,
            kind,
            fields,
        }
    }

    /// Returns the zero-based source declaration index.
    ///
    /// # Returns
    ///
    /// Returns the variant's position in its source enum.
    #[must_use]
    #[inline]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Returns the Rust variant name.
    ///
    /// # Returns
    ///
    /// Returns the source identifier.
    #[must_use]
    #[inline]
    pub const fn rust_name(&self) -> &'static str {
        self.rust_name
    }

    /// Returns the lookup name.
    ///
    /// # Returns
    ///
    /// Returns the reflection query name.
    #[must_use]
    #[inline]
    pub const fn query_name(&self) -> &'static str {
        self.query_name
    }

    /// Returns the declared variant shape.
    ///
    /// # Returns
    ///
    /// Returns the source variant kind.
    #[must_use]
    #[inline]
    pub const fn kind(&self) -> VariantKind {
        self.kind
    }

    /// Returns the fields in source order.
    ///
    /// # Returns
    ///
    /// Returns this variant's source field metadata.
    #[must_use]
    #[inline]
    pub const fn fields(&self) -> &'static [FieldDefinitionDescriptor] {
        self.fields
    }
}
