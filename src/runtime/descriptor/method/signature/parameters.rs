// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// A read-only, source-ordered view of a method's non-receiver parameters.
#[derive(Clone, Copy)]
pub struct Parameters<'ctx> {
    /// Static declaration facts shared by descriptors created in a reflection
    /// context.
    metadata: &'static [ParameterMetadata],
    /// Returns the reflection context required to resolve this descriptor’s
    /// referenced types.
    context: &'ctx ReflectionContext,
}

impl<'ctx> Parameters<'ctx> {
    /// Creates static metadata from the declaration facts supplied by
    /// registration. # Parameters
    ///
    /// - `metadata`: Static method declaration facts resolved in the reflection
    ///   context.
    /// - `context`: Reflection context used to resolve source expressions and
    ///   type metadata.
    /// # Parameters
    ///
    /// - `metadata`: Static method declaration facts resolved for this
    ///   reflection context.
    /// - `context`: Reflection context used to resolve this descriptor’s source
    ///   expressions.
    pub(super) const fn new(metadata: &'static [ParameterMetadata], context: &'ctx ReflectionContext) -> Self {
        Self { metadata, context }
    }

    /// Returns the reflection context that owns this parameter collection.
    pub fn context(self) -> &'ctx ReflectionContext {
        self.context
    }

    /// Returns the number of declared parameters.
    pub const fn len(self) -> usize {
        self.metadata.len()
    }

    /// Returns whether the method declares no non-receiver parameters.
    pub const fn is_empty(self) -> bool {
        self.metadata.is_empty()
    }

    /// Iterates over parameters in source declaration order.
    pub fn iter(self) -> impl ExactSizeIterator<Item = ParameterDescriptor<'ctx>> + 'ctx {
        self.metadata
            .iter()
            .map(move |metadata| ParameterDescriptor::new(metadata, self.context))
    }

    /// Returns the parameter at its zero-based source position, if present.
    ///
    /// # Parameters
    ///
    /// - `index`: Zero-based field position to inspect.
    pub fn get_at(self, index: usize) -> Option<ParameterDescriptor<'ctx>> {
        self.metadata
            .get(index)
            .map(|metadata| ParameterDescriptor::new(metadata, self.context))
    }

    /// Returns a uniquely named identifier parameter, if present.
    ///
    /// Parameters written with wildcard or destructuring patterns have no name.
    /// A duplicate name in malformed metadata is treated as ambiguous and
    /// returns `None`.
    ///
    /// # Parameters
    ///
    /// - `name`: Reflected field name to match against the target.
    pub fn get(self, name: &str) -> Option<ParameterDescriptor<'ctx>> {
        let mut matches = self.metadata.iter().filter(
            |metadata| matches!(metadata.pattern, ParameterPattern::Identifier(candidate) if candidate == name),
        );
        let first = matches.next()?;
        matches
            .next()
            .is_none()
            .then(|| ParameterDescriptor::new(first, self.context))
    }
}
