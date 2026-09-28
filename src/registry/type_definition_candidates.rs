// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Borrowed candidates for generic type-definition queries.

use crate::descriptor::TypeDefinitionDescriptor;

/// A borrowed, deterministic set of generic declaration matches.
///
/// # Examples
///
/// ```
/// use qubit_reflect::registry::RegistrySnapshotBuilder;
/// let registry = RegistrySnapshotBuilder::new().build().expect("empty snapshot is valid");
/// let candidates = registry.find_definitions_by_query_name("NoSuchDefinition");
/// assert!(candidates.is_empty());
/// ```
#[derive(Clone, Copy, Debug)]
pub struct TypeDefinitionCandidates<'registry> {
    /// Registry-owned generic declarations matching the requested name.
    descriptors: &'registry [&'static TypeDefinitionDescriptor],
}

impl<'registry> TypeDefinitionCandidates<'registry> {
    /// Creates a candidate view over one registry-owned slice.
    ///
    /// # Parameters
    ///
    /// - `descriptors`: Matching generic declarations in stable order.
    ///
    /// # Returns
    ///
    /// Returns a borrowed candidate view.
    pub(crate) const fn new(descriptors: &'registry [&'static TypeDefinitionDescriptor]) -> Self {
        Self { descriptors }
    }

    /// Returns candidates in stable fragment order.
    ///
    /// # Returns
    ///
    /// Returns an exact-size iterator over matching declarations.
    #[must_use]
    #[inline]
    pub fn iter(self) -> impl ExactSizeIterator<Item = &'static TypeDefinitionDescriptor> + 'registry {
        self.descriptors.iter().copied()
    }

    /// Returns the number of matching declarations.
    ///
    /// # Returns
    ///
    /// Returns the number of candidates.
    #[must_use]
    #[inline]
    pub const fn len(self) -> usize {
        self.descriptors.len()
    }

    /// Returns whether no declaration matched.
    ///
    /// # Returns
    ///
    /// Returns `true` when the candidate set is empty.
    #[must_use]
    #[inline]
    pub const fn is_empty(self) -> bool {
        self.descriptors.is_empty()
    }

    /// Returns the sole matching declaration, or `None` when absent or
    /// ambiguous.
    ///
    /// # Returns
    ///
    /// Returns the sole candidate, or `None` when the count is not one.
    #[must_use]
    #[inline]
    pub fn only(self) -> Option<&'static TypeDefinitionDescriptor> {
        (self.descriptors.len() == 1).then(|| self.descriptors[0])
    }
}

impl<'registry> IntoIterator for TypeDefinitionCandidates<'registry> {
    type Item = &'static TypeDefinitionDescriptor;
    type IntoIter = std::iter::Copied<std::slice::Iter<'registry, &'static TypeDefinitionDescriptor>>;

    /// Iterates over declarations in stable fragment order.
    ///
    /// # Returns
    ///
    /// Returns an exact-size iterator over the candidates.
    fn into_iter(self) -> Self::IntoIter {
        self.descriptors.iter().copied()
    }
}
