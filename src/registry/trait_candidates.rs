// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Borrowed candidates for diagnostic trait-path queries.

use crate::descriptor::TraitDefinitionDescriptor;

/// An ordered borrowed view over trait declarations sharing one diagnostic
/// path.
///
/// # Examples
///
/// ```
/// use qubit_reflect::registry::RegistrySnapshotBuilder;
/// let registry = RegistrySnapshotBuilder::new().build().expect("empty snapshot is valid");
/// let candidates = registry.find_trait_definitions_by_path("example::Missing");
/// assert!(candidates.is_empty());
/// ```
#[derive(Clone, Copy, Debug)]
pub struct TraitCandidates<'registry> {
    /// Registry-owned trait declarations matching one Rust path.
    descriptors: &'registry [&'static TraitDefinitionDescriptor],
}

impl<'registry> TraitCandidates<'registry> {
    /// Creates a borrowed candidate view over a registry-owned slice.
    ///
    /// # Parameters
    ///
    /// - `descriptors`: Matching declarations in stable order.
    ///
    /// # Returns
    ///
    /// Returns a borrowed candidate view.
    pub(crate) const fn new(descriptors: &'registry [&'static TraitDefinitionDescriptor]) -> Self {
        Self { descriptors }
    }

    /// Returns candidates in stable fragment order.
    ///
    /// # Returns
    ///
    /// Returns an exact-size iterator over matching trait declarations.
    #[must_use]
    #[inline]
    pub fn iter(self) -> impl ExactSizeIterator<Item = &'static TraitDefinitionDescriptor> + 'registry {
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

    /// Returns whether no declarations match.
    ///
    /// # Returns
    ///
    /// Returns `true` when the candidate set is empty.
    #[must_use]
    #[inline]
    pub const fn is_empty(self) -> bool {
        self.descriptors.is_empty()
    }

    /// Returns the sole matching declaration, rejecting ambiguous path lookups.
    ///
    /// # Returns
    ///
    /// Returns the sole candidate, or `None` when absent or ambiguous.
    #[must_use]
    #[inline]
    pub fn only(self) -> Option<&'static TraitDefinitionDescriptor> {
        (self.descriptors.len() == 1).then(|| self.descriptors[0])
    }
}

impl<'registry> IntoIterator for TraitCandidates<'registry> {
    type Item = &'static TraitDefinitionDescriptor;
    type IntoIter = std::iter::Copied<std::slice::Iter<'registry, &'static TraitDefinitionDescriptor>>;

    /// Iterates over candidates in stable fragment order.
    ///
    /// # Returns
    ///
    /// Returns an exact-size iterator over the candidates.
    fn into_iter(self) -> Self::IntoIter {
        self.descriptors.iter().copied()
    }
}
