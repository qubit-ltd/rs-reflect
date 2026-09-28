// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! return descriptor definitions.

use super::return_kind::ReturnKind;
use crate::descriptor::TypeDescriptor;
use crate::descriptor::TypeDescriptorResolver;
use crate::expression::TypeExpression;

/// The return declaration of a reflected method.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::{ReturnDescriptor, ReturnKind};
/// let output = ReturnDescriptor::new(ReturnKind::Unit, None, None);
/// assert_eq!(output.kind(), ReturnKind::Unit);
/// ```
#[derive(Clone, Debug)]
pub struct ReturnDescriptor {
    /// Structural category of the return value.
    kind: ReturnKind,
    /// Declared return type expression, when one is needed.
    pub(in crate::descriptor::method_descriptor) signature_type: Option<TypeExpression>,
    /// Resolver for an exact reflected root, when available.
    concrete_type: Option<TypeDescriptorResolver>,
}

impl ReturnDescriptor {
    /// Creates immutable return facts.
    ///
    /// `signature_type` is absent for unit and never returns when their
    /// [`ReturnKind`] is sufficient. `concrete_type` is present only for an
    /// exact reflected root.
    ///
    /// # Parameters
    ///
    /// - `kind`: Structural return category.
    /// - `signature_type`: Declared type expression, when applicable.
    /// - `concrete_type`: Exact reflected type resolver, when available.
    ///
    /// # Returns
    ///
    /// Returns immutable facts for the declared return value.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(
        kind: ReturnKind,
        signature_type: Option<TypeExpression>,
        concrete_type: Option<TypeDescriptorResolver>,
    ) -> Self {
        Self {
            kind,
            signature_type,
            concrete_type,
        }
    }

    /// Creates a unit return descriptor.
    ///
    /// # Returns
    ///
    /// Returns a descriptor representing `()`.
    #[must_use]
    pub const fn unit() -> Self {
        Self::new(ReturnKind::Unit, None, None)
    }

    /// Returns the structural return category.
    ///
    /// # Returns
    ///
    /// Returns the return category.
    #[must_use]
    #[inline]
    pub const fn kind(&self) -> ReturnKind {
        self.kind
    }

    /// Returns the declared return type expression.
    ///
    /// `None` means the unit or never category carries the complete fact.
    ///
    /// # Returns
    ///
    /// Returns the declared type expression, or `None` for unit/never.
    #[must_use]
    #[inline]
    pub const fn signature_type(&self) -> Option<&TypeExpression> {
        self.signature_type.as_ref()
    }

    /// Returns the exact reflected return type when it is known.
    ///
    /// `None` denotes unit, never, a reference, opaque output, or an unresolved
    /// symbolic type.
    ///
    /// # Returns
    ///
    /// Returns the exact reflected type root, or `None` when unavailable.
    #[must_use]
    pub fn concrete_type(&self) -> Option<&'static TypeDescriptor> {
        self.concrete_type.map(|resolver| resolver())
    }
}
