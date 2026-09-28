// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! parameter descriptor definitions.

use super::parameter_passing_mode::ParameterPassingMode;
use super::parameter_pattern_descriptor::ParameterPatternDescriptor;
use crate::descriptor::TypeDescriptor;
use crate::descriptor::TypeDescriptorResolver;
use crate::expression::TypeExpression;

/// One non-receiver method parameter in declaration order.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::{ParameterDescriptor, ParameterPassingMode, ParameterPatternDescriptor};
/// use qubit_reflect::expression::{ConcreteTypeExpression, TypeExpression};
///
/// let parameter = ParameterDescriptor::new(
///     0,
///     Some("value"),
///     ParameterPatternDescriptor::Identifier,
///     ParameterPassingMode::Owned,
///     TypeExpression::Concrete(
///         ConcreteTypeExpression::new(["u8"], []).expect("non-empty path"),
///     ),
///     None,
/// );
/// assert_eq!(parameter.name(), Some("value"));
/// ```
#[derive(Clone, Debug)]
pub struct ParameterDescriptor {
    /// Zero-based position among non-receiver parameters.
    index: usize,
    /// Bindable identifier, absent for wildcard and destructuring patterns.
    name: Option<&'static str>,
    /// Source pattern category retained independently of parser syntax.
    pattern: ParameterPatternDescriptor,
    /// Ownership or borrowing mode at the method boundary.
    passing_mode: ParameterPassingMode,
    /// Declared type expression, which may still be symbolic.
    pub(in crate::descriptor::method_descriptor) signature_type: TypeExpression,
    /// Resolver for an exact reflected root, when available.
    concrete_type: Option<TypeDescriptorResolver>,
}

impl ParameterDescriptor {
    /// Creates immutable parameter facts.
    ///
    /// `index` excludes the receiver. `name` must be `None` for wildcard and
    /// destructuring patterns. `concrete_type` is present only when the
    /// declaration can navigate to an exact reflected root.
    ///
    /// # Parameters
    ///
    /// - `index`: Zero-based non-receiver parameter position.
    /// - `name`: Bindable identifier, or `None` for wildcard/destructuring.
    /// - `pattern`: Source pattern category.
    /// - `passing_mode`: Whether the method owns or borrows the argument.
    /// - `signature_type`: Declared type expression.
    /// - `concrete_type`: Exact type resolver, when available.
    ///
    /// # Returns
    ///
    /// Returns immutable parameter facts for the declaration.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(
        index: usize,
        name: Option<&'static str>,
        pattern: ParameterPatternDescriptor,
        passing_mode: ParameterPassingMode,
        signature_type: TypeExpression,
        concrete_type: Option<TypeDescriptorResolver>,
    ) -> Self {
        Self {
            index,
            name,
            pattern,
            passing_mode,
            signature_type,
            concrete_type,
        }
    }

    /// Returns the zero-based non-receiver parameter index.
    ///
    /// # Returns
    ///
    /// Returns the parameter's position after excluding the receiver.
    #[must_use]
    #[inline]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Returns the identifier used for named binding.
    ///
    /// `None` denotes a wildcard or destructuring pattern.
    ///
    /// # Returns
    ///
    /// Returns the bindable identifier, or `None` when the pattern has none.
    #[must_use]
    #[inline]
    pub const fn name(&self) -> Option<&'static str> {
        self.name
    }

    /// Returns the parser-independent source pattern category.
    ///
    /// # Returns
    ///
    /// Returns the parameter's source pattern classification.
    #[must_use]
    #[inline]
    pub const fn pattern(&self) -> &ParameterPatternDescriptor {
        &self.pattern
    }

    /// Returns how the argument crosses the method boundary.
    ///
    /// # Returns
    ///
    /// Returns whether the argument is owned, shared-borrowed, or mutably
    /// borrowed.
    #[must_use]
    #[inline]
    pub const fn passing_mode(&self) -> ParameterPassingMode {
        self.passing_mode
    }

    /// Returns the declared, possibly symbolic parameter type.
    ///
    /// # Returns
    ///
    /// Returns the structural type expression from the signature.
    #[must_use]
    #[inline]
    pub const fn signature_type(&self) -> &TypeExpression {
        &self.signature_type
    }

    /// Returns the exact reflected parameter type when it is known.
    ///
    /// `None` denotes a symbolic, opaque, or otherwise unresolved type.
    ///
    /// # Returns
    ///
    /// Returns the exact reflected type root, or `None` when unavailable.
    #[must_use]
    pub fn concrete_type(&self) -> Option<&'static TypeDescriptor> {
        self.concrete_type.map(|resolver| resolver())
    }
}
