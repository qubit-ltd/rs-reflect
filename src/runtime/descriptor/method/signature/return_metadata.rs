// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Static return metadata for a method declaration.
#[derive(Clone, Copy)]
pub struct ReturnMetadata {
    /// Returns the declared return-value category.
    pub(crate) kind: ReturnKind,
    /// Retains the source type expression used to describe this receiver.
    pub(crate) expression: Option<&'static ExpressionData>,
    /// Optional metadata for the exact Rust type known at registration time.
    pub(crate) concrete_type: Option<fn() -> &'static crate::runtime::descriptor::metadata::TypeMetadata>,
}

impl ReturnMetadata {
    /// Creates unit return metadata.
    pub const fn unit() -> Self {
        Self {
            kind: ReturnKind::Unit,
            expression: None,
            concrete_type: None,
        }
    }

    /// Creates metadata for a non-unit return value.
    ///
    /// # Parameters
    ///
    /// - `kind`: Receiver form, return category, or constant type being
    ///   recorded.
    /// - `expression`: Source type expression retained for context-bound
    ///   resolution.
    pub const fn new(kind: ReturnKind, expression: &'static ExpressionData) -> Self {
        Self {
            kind,
            expression: Some(expression),
            concrete_type: None,
        }
    }

    /// Records the exact concrete Rust type when the return value has one.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    pub const fn with_concrete_type<T: crate::runtime::descriptor::Reflect>(mut self) -> Self {
        self.concrete_type = Some(crate::runtime::descriptor::metadata::TypeMetadata::of::<T>);
        self
    }
}
