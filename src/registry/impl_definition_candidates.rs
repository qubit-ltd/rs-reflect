// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Owned candidate sets for structural impl-target queries.

use std::vec::IntoIter;

use crate::descriptor::ImplDefinitionDescriptor;

/// An owned, deterministic set of references to impl definitions matching
/// one symbolic target expression.
///
/// # Examples
///
/// ```
/// use qubit_reflect::expression::{ConcreteTypeExpression, TypeExpression};
/// use qubit_reflect::registry::RegistrySnapshotBuilder;
///
/// let registry = RegistrySnapshotBuilder::new().build().expect("empty snapshot is valid");
/// let target = TypeExpression::Concrete(
///     ConcreteTypeExpression::new(["example", "Missing"], []).expect("non-empty path"),
/// );
/// let candidates = registry.find_impl_definitions_by_target(&target);
/// assert!(candidates.is_empty());
/// ```
#[derive(Clone, Debug)]
pub struct ImplDefinitionCandidates {
    /// Matching declarations retained in stable source-fragment order.
    descriptors: Box<[&'static ImplDefinitionDescriptor]>,
}

impl ImplDefinitionCandidates {
    /// Creates a candidate set from declarations already ordered by the
    /// registry.
    ///
    /// # Parameters
    ///
    /// - `descriptors`: Matching declarations in stable source-fragment order.
    ///
    /// # Returns
    ///
    /// Returns an owning candidate set.
    pub(crate) fn new(descriptors: Box<[&'static ImplDefinitionDescriptor]>) -> Self {
        Self { descriptors }
    }

    /// Iterates over matching definitions in stable fragment order.
    ///
    /// # Returns
    ///
    /// Returns an exact-size iterator over matching declarations.
    #[must_use]
    #[inline]
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &'static ImplDefinitionDescriptor> + '_ {
        self.descriptors.iter().copied()
    }

    /// Returns the number of matching definitions.
    ///
    /// # Returns
    ///
    /// Returns the number of candidates.
    #[must_use]
    #[inline]
    pub const fn len(&self) -> usize {
        self.descriptors.len()
    }

    /// Returns whether no impl definition has the requested target.
    ///
    /// # Returns
    ///
    /// Returns `true` when no declaration matched.
    #[must_use]
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.descriptors.is_empty()
    }
}

impl IntoIterator for ImplDefinitionCandidates {
    type Item = &'static ImplDefinitionDescriptor;
    type IntoIter = IntoIter<&'static ImplDefinitionDescriptor>;

    /// Iterates over matching definitions in stable fragment order.
    ///
    /// # Returns
    ///
    /// Returns an owning iterator over the matching declarations.
    fn into_iter(self) -> Self::IntoIter {
        self.descriptors.into_vec().into_iter()
    }
}
