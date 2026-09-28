// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Borrowed candidates for concrete type-name queries.

use crate::descriptor::TypeDescriptor;

/// A borrowed, deterministic set of type-descriptor name matches.
///
/// # Examples
///
/// ```
/// use qubit_reflect::registry::RegistrySnapshotBuilder;
/// let registry = RegistrySnapshotBuilder::new().build().expect("empty snapshot is valid");
/// let candidates = registry.find_by_type_name("u8");
/// assert!(candidates.is_empty());
/// ```
#[derive(Clone, Copy, Debug)]
pub struct TypeCandidates<'registry> {
    /// Registry-owned roots matching the requested name in stable order.
    descriptors: &'registry [&'static TypeDescriptor],
}

impl<'registry> TypeCandidates<'registry> {
    /// Creates a borrowed candidate view over a registry-owned slice.
    ///
    /// # Parameters
    ///
    /// - `descriptors`: Matching roots in stable order.
    ///
    /// # Returns
    ///
    /// Returns a borrowed candidate view.
    pub(crate) const fn new(descriptors: &'registry [&'static TypeDescriptor]) -> Self {
        Self { descriptors }
    }

    /// Returns candidates in stable fragment order.
    ///
    /// # Returns
    ///
    /// Returns an exact-size iterator over matching roots.
    #[must_use]
    #[inline]
    pub fn iter(self) -> impl ExactSizeIterator<Item = &'static TypeDescriptor> + 'registry {
        self.descriptors.iter().copied()
    }

    /// Returns the number of matching descriptors.
    ///
    /// # Returns
    ///
    /// Returns the number of candidates.
    #[must_use]
    #[inline]
    pub const fn len(self) -> usize {
        self.descriptors.len()
    }

    /// Returns whether no descriptor matched the requested name.
    ///
    /// # Returns
    ///
    /// Returns `true` when the candidate set is empty.
    #[must_use]
    #[inline]
    pub const fn is_empty(self) -> bool {
        self.descriptors.is_empty()
    }
}

impl<'registry> IntoIterator for TypeCandidates<'registry> {
    type Item = &'static TypeDescriptor;
    type IntoIter = std::iter::Copied<std::slice::Iter<'registry, &'static TypeDescriptor>>;

    /// Iterates over candidates in stable fragment order.
    ///
    /// # Returns
    ///
    /// Returns an exact-size iterator over the candidates.
    fn into_iter(self) -> Self::IntoIter {
        self.descriptors.iter().copied()
    }
}
