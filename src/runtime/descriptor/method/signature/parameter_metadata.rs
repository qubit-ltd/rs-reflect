// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Static facts about one declared parameter.
#[derive(Clone, Copy)]
pub struct ParameterMetadata {
    /// Returns the parameter’s zero-based position, excluding the receiver.
    pub(crate) index: usize,
    /// Returns the source binding pattern retained during registration.
    pub(crate) pattern: ParameterPattern,
    /// Returns the argument passing mode declared by the method.
    pub(crate) passing: PassingMode,
    /// Retains the source type expression used to describe this receiver.
    pub(crate) expression: &'static ExpressionData,
    /// Optional metadata for the exact Rust type known at registration time.
    pub(crate) concrete_type: Option<fn() -> &'static crate::runtime::descriptor::metadata::TypeMetadata>,
}

impl ParameterMetadata {
    /// Creates static metadata for one method parameter.
    ///
    /// # Parameters
    ///
    /// - `index`: Zero-based field position to inspect.
    /// - `pattern`: Source binding pattern associated with the parameter.
    /// - `passing`: Argument passing mode declared by the method.
    /// - `expression`: Source type expression retained for context-bound
    ///   resolution.
    pub const fn new(
        index: usize,
        pattern: ParameterPattern,
        passing: PassingMode,
        expression: &'static ExpressionData,
    ) -> Self {
        Self {
            index,
            pattern,
            passing,
            expression,
            concrete_type: None,
        }
    }

    /// Records the exact concrete Rust type when the parameter has one.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    pub const fn with_concrete_type<T: ?Sized + 'static>(mut self) -> Self {
        self.concrete_type = Some(crate::runtime::descriptor::metadata::TypeMetadata::of::<T>);
        self
    }

    /// Returns the concrete type identity recorded for this parameter, if any.
    pub(crate) fn concrete_type_id(&self) -> Option<TypeId> {
        self.concrete_type.map(|metadata| metadata().type_id())
    }

    /// Returns this parameter's zero-based position in the call signature.
    pub(crate) const fn index(&self) -> usize {
        self.index
    }

    /// Returns the passing mode recorded for this parameter.
    pub(crate) const fn passing(&self) -> PassingMode {
        self.passing
    }

    /// Returns the source pattern recorded for this parameter.
    pub(crate) const fn pattern(&self) -> ParameterPattern {
        self.pattern
    }
}
