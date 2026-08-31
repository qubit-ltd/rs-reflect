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
    variant_index: Option<usize>,
    variant_rust_name: Option<&'static str>,
}

impl FieldIdentity {
    /// Creates the identity attached to a generated field adapter error.
    ///
    /// The type ID and diagnostic name must describe the same declaring root.
    #[doc(hidden)]
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
            variant_index: None,
            variant_rust_name: None,
        }
    }

    /// Creates the identity attached to a generated enum-variant field error.
    ///
    /// `variant_index` and `variant_rust_name` identify the same source variant
    /// and ensure equal field positions in different variants stay distinct.
    #[doc(hidden)]
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
            variant_index: Some(variant_index),
            variant_rust_name: Some(variant_rust_name),
        }
    }

    /// Returns the declaring root's process-local Rust type identity.
    #[must_use]
    #[inline(always)]
    pub const fn declaring_type(&self) -> TypeId {
        self.declaring_type
    }

    /// Returns the declaring root's diagnostic Rust type name.
    #[must_use]
    #[inline(always)]
    pub const fn declaring_type_name(&self) -> &'static str {
        self.declaring_type_name
    }

    /// Returns the field's zero-based source declaration index.
    #[must_use]
    #[inline(always)]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Returns the source Rust name, or `None` for positional fields.
    #[must_use]
    #[inline(always)]
    pub const fn rust_name(&self) -> Option<&'static str> {
        self.rust_name
    }

    /// Returns the containing variant's source index for an enum field.
    ///
    /// `None` identifies a direct struct field.
    #[must_use]
    #[inline(always)]
    pub const fn variant_index(&self) -> Option<usize> {
        self.variant_index
    }

    /// Returns the containing variant's Rust name for an enum field.
    ///
    /// `None` identifies a direct struct field.
    #[must_use]
    #[inline(always)]
    pub const fn variant_rust_name(&self) -> Option<&'static str> {
        self.variant_rust_name
    }
}

impl fmt::Display for FieldIdentity {
    /// Formats the declaring type and source field identity without using a
    /// query alias as the source name.
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
