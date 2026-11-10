// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Metadata for the receiver parameter.
#[derive(Clone, Copy)]
pub struct ReceiverDescriptor {
    /// Returns the declared return-value category.
    pub(crate) kind: ReceiverKind,
    /// Provides the concrete receiver type metadata when the declaration is
    /// statically typed.
    pub(crate) type_metadata: Option<fn() -> &'static crate::runtime::descriptor::metadata::TypeMetadata>,
    /// Retains the source type expression used to describe this receiver.
    pub(crate) expression: Option<&'static ExpressionData>,
}

impl ReceiverDescriptor {
    /// Returns the receiver form declared by the method.
    pub const fn kind(self) -> ReceiverKind {
        self.kind
    }

    /// Creates receiver metadata for one concrete reflected target type.
    ///
    /// # Parameters
    ///
    /// - `kind`: Receiver form, return category, or constant type being
    ///   recorded.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    ///
    ///
    /// # Parameters
    ///
    /// - `kind`: Receiver form, return category, or constant type being
    ///   recorded.
    pub const fn for_type<T: crate::runtime::descriptor::Reflect>(kind: ReceiverKind) -> Self {
        Self {
            kind,
            type_metadata: Some(crate::runtime::descriptor::metadata::TypeMetadata::of::<T>),
            expression: None,
        }
    }

    /// Creates receiver metadata whose concrete type is supplied by an
    /// effective method.
    ///
    /// # Parameters
    ///
    /// - `kind`: Receiver form, return category, or constant type being
    ///   recorded.
    /// - `expression`: Source type expression retained for context-bound
    ///   resolution.
    pub const fn trait_self(kind: ReceiverKind, expression: &'static ExpressionData) -> Self {
        Self {
            kind,
            type_metadata: None,
            expression: Some(expression),
        }
    }

    /// Binds an unbound receiver declaration to its effective target type.
    ///
    /// # Parameters
    ///
    /// - `target`: Target type whose effective method metadata is being
    ///   resolved.
    pub(crate) const fn for_target(
        mut self,
        target: fn() -> &'static crate::runtime::descriptor::metadata::TypeMetadata,
    ) -> Self {
        if self.type_metadata.is_none() {
            self.type_metadata = Some(target);
        }
        self
    }

    /// Retains the source type expression for an explicitly typed receiver.
    ///
    /// # Parameters
    ///
    /// - `expression`: Source type expression retained for context-bound
    ///   resolution.
    pub const fn with_expression(mut self, expression: &'static ExpressionData) -> Self {
        self.expression = Some(expression);
        self
    }
}
