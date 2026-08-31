// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Lazily resolved ordered lists of type relationships.

use std::fmt;
use std::sync::OnceLock;

use crate::__private::LazyTypeRef;
use crate::descriptor::TypeRef;

/// An ordered relationship list resolved and frozen on first navigation.
pub(crate) struct LazyTypeRefList {
    references: &'static [LazyTypeRef],
    resolved: OnceLock<Box<[TypeRef]>>,
}

impl LazyTypeRefList {
    /// Creates a deferred list from source-order lazy relationships.
    #[must_use]
    pub(crate) const fn new(references: &'static [LazyTypeRef]) -> Self {
        Self {
            references,
            resolved: OnceLock::new(),
        }
    }

    /// Returns the number of relationships without resolving any target.
    #[must_use]
    #[inline(always)]
    pub(crate) const fn len(&self) -> usize {
        self.references.len()
    }

    /// Returns all targets in source order, resolving the list once.
    ///
    /// Concurrent callers receive the same immutable slice. A resolver panic
    /// propagates and leaves the list available for a later retry.
    #[must_use]
    #[inline]
    pub(crate) fn get(&'static self) -> &'static [TypeRef] {
        self.resolved
            .get_or_init(|| {
                self.references
                    .iter()
                    .map(|reference| reference.get().clone())
                    .collect()
            })
            .as_ref()
    }
}

impl fmt::Debug for LazyTypeRefList {
    /// Formats cached state without forcing any relationship to resolve.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.resolved.get() {
            Some(references) => formatter.debug_tuple("LazyTypeRefList").field(references).finish(),
            None => formatter
                .debug_struct("LazyTypeRefList")
                .field("length", &self.references.len())
                .field("state", &"<unresolved>")
                .finish(),
        }
    }
}
