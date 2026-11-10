// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// A context-bound view of a method's declared return value.
#[derive(Clone, Copy)]
pub struct ReturnDescriptor<'ctx> {
    /// Static declaration facts shared by descriptors created in a reflection
    /// context.
    metadata: &'static ReturnMetadata,
    /// Returns the reflection context required to resolve this descriptor’s
    /// referenced types.
    context: &'ctx ReflectionContext,
}

impl<'ctx> ReturnDescriptor<'ctx> {
    /// Creates static metadata from the declaration facts supplied by
    /// registration. # Parameters
    ///
    /// - `context`: Reflection context used to resolve source expressions and
    ///   type metadata.
    /// # Parameters
    ///
    /// - `context`: Reflection context used to resolve this descriptor’s source
    ///   expressions.
    pub(crate) const fn new(metadata: &'static ReturnMetadata, context: &'ctx ReflectionContext) -> Self {
        Self { metadata, context }
    }
    /// Returns the reflection context that owns this descriptor.
    pub fn context(self) -> &'ctx ReflectionContext {
        self.context
    }
    /// Returns the declared return-value category.
    pub const fn kind(self) -> ReturnKind {
        self.metadata.kind
    }
    /// Returns the source-level return type expression when one was declared.
    pub fn type_expression(self) -> Option<TypeExpression<'ctx>> {
        self.metadata
            .expression
            .map(|expression| TypeExpression::new(expression, self.context))
    }
    /// Returns the concrete Rust return type when registration retained one.
    pub fn find_concrete_type(self) -> Option<TypeDescriptor<'ctx>> {
        self.metadata
            .concrete_type
            .map(|metadata| TypeDescriptor::new(metadata(), self.context))
    }
}
