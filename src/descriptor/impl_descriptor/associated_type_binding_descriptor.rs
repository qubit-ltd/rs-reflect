// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! AssociatedTypeBindingDescriptor metadata and behavior.

use crate::descriptor::AssociatedTypeDescriptor;
use crate::descriptor::TypeDescriptor;
use crate::descriptor::TypeDescriptorResolver;
use crate::expression::TypeExpression;

/// One associated type binding contributed by a concrete impl.
///
/// This descriptor is constructed by generated registration code.
///
/// # Examples
///
/// ```
/// use std::sync::LazyLock;
/// use qubit_reflect::descriptor::{AssociatedTypeBindingDescriptor, AssociatedTypeDescriptor};
/// use qubit_reflect::expression::{ConcreteTypeExpression, TypeExpression};
///
/// static DECLARATION: LazyLock<AssociatedTypeDescriptor> = LazyLock::new(|| {
///     AssociatedTypeDescriptor::new(0, "Item", "item", Box::new([]), None)
/// });
/// let binding = AssociatedTypeBindingDescriptor::new(
///     &DECLARATION,
///     TypeExpression::Concrete(ConcreteTypeExpression::new(["u8"], []).expect("non-empty path")),
///     None,
/// );
/// assert_eq!(binding.declaration().rust_name(), "Item");
/// ```
#[must_use]
#[derive(Clone, Debug)]
pub struct AssociatedTypeBindingDescriptor {
    /// Associated type declaration being implemented.
    declaration: &'static AssociatedTypeDescriptor,
    /// Concrete or symbolic assigned type expression.
    value: TypeExpression,
    /// Resolver for the exact type, when it can be resolved.
    concrete_type: Option<TypeDescriptorResolver>,
}

impl AssociatedTypeBindingDescriptor {
    /// Creates an associated type binding.
    ///
    /// `concrete_type` is present only when `value` resolves to an exact root.
    ///
    /// # Parameters
    ///
    /// - `declaration`: Associated type declaration being bound.
    /// - `value`: Concrete or symbolic assigned type expression.
    /// - `concrete_type`: Exact reflected type resolver, when known.
    ///
    /// # Returns
    ///
    /// Returns the associated type binding facts.
    #[doc(hidden)]
    #[must_use = "the associated type binding facts are required by generated registration"]
    pub const fn new(
        declaration: &'static AssociatedTypeDescriptor,
        value: TypeExpression,
        concrete_type: Option<TypeDescriptorResolver>,
    ) -> Self {
        Self {
            declaration,
            value,
            concrete_type,
        }
    }

    /// Returns the trait declaration being bound.
    ///
    /// # Returns
    ///
    /// Returns the associated type declaration.
    #[must_use]
    #[inline]
    pub const fn declaration(&self) -> &'static AssociatedTypeDescriptor {
        self.declaration
    }

    /// Returns the concrete or still-symbolic binding expression.
    ///
    /// # Returns
    ///
    /// Returns the assigned type expression.
    #[must_use]
    #[inline]
    pub const fn value(&self) -> &TypeExpression {
        &self.value
    }

    /// Returns the exact reflected binding when it is known.
    ///
    /// `None` means the expression remains symbolic or unresolved.
    ///
    /// # Returns
    ///
    /// Returns the resolved root descriptor, or `None` when unresolved.
    #[must_use]
    pub fn concrete_type(&self) -> Option<&'static TypeDescriptor> {
        self.concrete_type.map(|resolver| resolver())
    }
}
