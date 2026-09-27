// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Runtime field identity retained by access errors.

use std::any::TypeId;
use std::fmt;

/// Stable runtime identity retained in every field access error.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct FieldIdentity {
    declaring_type: TypeId,
    declaring_type_name: &'static str,
    index: usize,
    rust_name: Option<&'static str>,
    query_name: Option<&'static str>,
    variant_index: Option<usize>,
    variant_rust_name: Option<&'static str>,
}

impl FieldIdentity {
    /// Creates the identity attached to a generated field adapter error.
    ///
    /// The type ID and diagnostic name must describe the same declaring root.
    ///
    /// # Parameters
    ///
    /// - `declaring_type`: Process-local identity of the declaring root type.
    /// - `declaring_type_name`: Stable diagnostic name of that same type.
    /// - `index`: Zero-based source index of the field.
    /// - `rust_name`: Source field name, or `None` for a positional field.
    ///
    /// # Returns
    ///
    /// A direct-field identity whose query name initially matches `rust_name`.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(
        declaring_type: TypeId,
        declaring_type_name: &'static str,
        index: usize,
        rust_name: Option<&'static str>,
    ) -> Self {
        Self {
            declaring_type,
            declaring_type_name,
            index,
            rust_name,
            query_name: rust_name,
            variant_index: None,
            variant_rust_name: None,
        }
    }

    /// Creates the identity attached to a generated enum-variant field error.
    ///
    /// `variant_index` and `variant_rust_name` identify the same source variant
    /// and ensure equal field positions in different variants stay distinct.
    ///
    /// # Parameters
    ///
    /// - `declaring_type`: Process-local identity of the declaring enum.
    /// - `declaring_type_name`: Diagnostic name of the declaring enum.
    /// - `index`: Zero-based source index of the field within its variant.
    /// - `rust_name`: Source field name, or `None` for a positional field.
    /// - `variant_index`: Zero-based source index of the containing variant.
    /// - `variant_rust_name`: Source name of the containing variant.
    ///
    /// # Returns
    ///
    /// An identity distinguishing this field from equal positions in other
    /// variants.
    #[doc(hidden)]
    #[must_use]
    pub const fn new_variant(
        declaring_type: TypeId,
        declaring_type_name: &'static str,
        index: usize,
        rust_name: Option<&'static str>,
        variant_index: usize,
        variant_rust_name: &'static str,
    ) -> Self {
        Self {
            declaring_type,
            declaring_type_name,
            index,
            rust_name,
            query_name: rust_name,
            variant_index: Some(variant_index),
            variant_rust_name: Some(variant_rust_name),
        }
    }

    /// Creates a direct-field identity with distinct Rust and query names.
    ///
    /// # Parameters
    ///
    /// - `declaring_type`: Process-local identity of the declaring root type.
    /// - `declaring_type_name`: Diagnostic name of the declaring root type.
    /// - `index`: Zero-based source index of the field.
    /// - `rust_name`: Source field name, or `None` for a positional field.
    /// - `query_name`: Name used by reflection lookup, if one is exposed.
    ///
    /// # Returns
    ///
    /// A direct-field identity retaining separate source and query names.
    #[doc(hidden)]
    #[must_use]
    pub const fn new_with_query_name(
        declaring_type: TypeId,
        declaring_type_name: &'static str,
        index: usize,
        rust_name: Option<&'static str>,
        query_name: Option<&'static str>,
    ) -> Self {
        Self {
            declaring_type,
            declaring_type_name,
            index,
            rust_name,
            query_name,
            variant_index: None,
            variant_rust_name: None,
        }
    }

    /// Creates a variant-field identity with distinct Rust and query names.
    ///
    /// # Parameters
    ///
    /// - `declaring_type`: Process-local identity of the declaring enum.
    /// - `declaring_type_name`: Diagnostic name of the declaring enum.
    /// - `index`: Zero-based source index of the field within its variant.
    /// - `rust_name`: Source field name, or `None` for a positional field.
    /// - `query_name`: Name used by reflection lookup, if one is exposed.
    /// - `variant_index`: Zero-based source index of the containing variant.
    /// - `variant_rust_name`: Source name of the containing variant.
    ///
    /// # Returns
    ///
    /// A variant-field identity retaining separate source and query names.
    #[doc(hidden)]
    #[must_use]
    pub const fn new_variant_with_query_name(
        declaring_type: TypeId,
        declaring_type_name: &'static str,
        index: usize,
        rust_name: Option<&'static str>,
        query_name: Option<&'static str>,
        variant_index: usize,
        variant_rust_name: &'static str,
    ) -> Self {
        Self {
            declaring_type,
            declaring_type_name,
            index,
            rust_name,
            query_name,
            variant_index: Some(variant_index),
            variant_rust_name: Some(variant_rust_name),
        }
    }

    /// Returns the declaring root's process-local Rust type identity.
    ///
    /// # Returns
    ///
    /// The `TypeId` shared by all fields declared on the same root type.
    #[must_use]
    #[inline]
    pub const fn declaring_type(&self) -> TypeId {
        self.declaring_type
    }

    /// Returns the declaring root's diagnostic Rust type name.
    ///
    /// # Returns
    ///
    /// The static diagnostic name supplied when this identity was created.
    #[must_use]
    #[inline]
    pub const fn declaring_type_name(&self) -> &'static str {
        self.declaring_type_name
    }

    /// Returns the field's zero-based source declaration index.
    ///
    /// # Returns
    ///
    /// The field's position in its struct or containing enum variant.
    #[must_use]
    #[inline]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Returns the source Rust name, or `None` for positional fields.
    ///
    /// # Returns
    ///
    /// The source identifier when the field is named.
    #[must_use]
    #[inline]
    pub const fn rust_name(&self) -> Option<&'static str> {
        self.rust_name
    }

    /// Returns the reflection query name used for lookup.
    ///
    /// # Returns
    ///
    /// The configured query name, or `None` when lookup is positional.
    #[must_use]
    #[inline]
    pub const fn query_name(&self) -> Option<&'static str> {
        self.query_name
    }

    /// Returns the containing variant's source index for an enum field.
    ///
    /// `None` identifies a direct struct field.
    ///
    /// # Returns
    ///
    /// The variant's source position, or `None` for a struct field.
    #[must_use]
    #[inline]
    pub const fn variant_index(&self) -> Option<usize> {
        self.variant_index
    }

    /// Returns the containing variant's Rust name for an enum field.
    ///
    /// `None` identifies a direct struct field.
    ///
    /// # Returns
    ///
    /// The variant's source name, or `None` for a struct field.
    #[must_use]
    #[inline]
    pub const fn variant_rust_name(&self) -> Option<&'static str> {
        self.variant_rust_name
    }
}

impl fmt::Display for FieldIdentity {
    /// Formats the declaring type and source field identity without using a
    /// query alias as the source name.
    ///
    /// # Parameters
    ///
    /// - `formatter`: Destination receiving the identity's diagnostic form.
    ///
    /// # Returns
    ///
    /// The formatter result after writing the source identity.
    ///
    /// # Errors
    ///
    /// Returns the formatter's error if the destination rejects the output.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.variant_rust_name, self.rust_name) {
            (Some(variant), Some(rust_name)) => {
                write!(formatter, "{}::{variant}.{rust_name}", self.declaring_type_name)
            }
            (Some(variant), None) => write!(
                formatter,
                "{}::{variant} field #{}",
                self.declaring_type_name, self.index
            ),
            (None, Some(rust_name)) => {
                write!(formatter, "{}::{rust_name}", self.declaring_type_name)
            }
            (None, None) => write!(formatter, "{} field #{}", self.declaring_type_name, self.index),
        }
    }
}
