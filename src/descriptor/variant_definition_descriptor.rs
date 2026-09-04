// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

//! Source-level enum variant facts for reflected generic declarations.

use crate::descriptor::FieldDefinitionDescriptor;
use crate::descriptor::VariantKind;

/// The immutable source-level description of one enum variant.
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
    #[must_use]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Returns the Rust variant name.
    #[must_use]
    pub const fn rust_name(&self) -> &'static str {
        self.rust_name
    }

    /// Returns the lookup name.
    #[must_use]
    pub const fn query_name(&self) -> &'static str {
        self.query_name
    }

    /// Returns the declared variant shape.
    #[must_use]
    pub const fn kind(&self) -> VariantKind {
        self.kind
    }

    /// Returns the fields in source order.
    #[must_use]
    pub const fn fields(&self) -> &'static [FieldDefinitionDescriptor] {
        self.fields
    }
}
