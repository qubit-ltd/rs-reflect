// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// A context-bound view of one declared method parameter.
#[derive(Clone, Copy)]
pub struct ParameterDescriptor<'ctx> {
    /// Static declaration facts shared by descriptors created in a reflection
    /// context.
    metadata: &'static ParameterMetadata,
    /// Returns the reflection context required to resolve this descriptor’s
    /// referenced types.
    context: &'ctx ReflectionContext,
}

impl<'ctx> ParameterDescriptor<'ctx> {
    /// Creates static metadata from the declaration facts supplied by
    /// registration. # Parameters
    ///
    /// - `context`: Reflection context used to resolve source expressions and
    ///   type metadata.
    /// # Parameters
    ///
    /// - `context`: Reflection context used to resolve this descriptor’s source
    ///   expressions.
    pub(crate) const fn new(metadata: &'static ParameterMetadata, context: &'ctx ReflectionContext) -> Self {
        Self { metadata, context }
    }
    /// Returns the reflection context that owns this descriptor.
    pub fn context(self) -> &'ctx ReflectionContext {
        self.context
    }
    /// Returns the parameter's zero-based position, excluding the receiver.
    pub const fn index(self) -> usize {
        self.metadata.index
    }
    /// Returns the source identifier, or `None` for wildcard and destructured
    /// patterns.
    pub const fn name(self) -> Option<&'static str> {
        match self.metadata.pattern {
            ParameterPattern::Identifier(name) => Some(name),
            _ => None,
        }
    }
    /// Returns the source binding pattern.
    pub const fn pattern(self) -> ParameterPattern {
        self.metadata.pattern
    }
    /// Returns how the parameter receives its value.
    pub const fn passing(self) -> PassingMode {
        self.metadata.passing
    }
    /// Returns the source-level type expression for this parameter.
    pub fn type_expression(self) -> TypeExpression<'ctx> {
        TypeExpression::new(self.metadata.expression, self.context)
    }
    /// Returns the concrete Rust type descriptor when registration retained
    /// one.
    pub fn find_concrete_type(self) -> Option<TypeDescriptor<'ctx>> {
        self.metadata
            .concrete_type
            .map(|metadata| TypeDescriptor::new(metadata(), self.context))
    }
}
