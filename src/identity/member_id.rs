// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Composite identities for reflected members.

use crate::identity::FragmentIdentity;

/// The stable composite identity of a reflected member.
///
/// # Examples
///
/// ```
/// use qubit_reflect::identity::{FragmentIdentity, MemberId};
/// let fragment = FragmentIdentity::new("example", "demo", 4, 1, "field", 12);
/// let member = MemberId::new("example::Record", "field", 0, fragment);
/// assert_eq!(member.kind(), "field");
/// ```
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MemberId {
    /// Stable identity of the descriptor declaring this member.
    declaring_identity: Box<str>,
    /// Member category such as `field`, `variant`, or `method`.
    kind: Box<str>,
    /// Zero-based declaration position within the category.
    index: usize,
    /// Registration fragment that contributed this member.
    fragment: FragmentIdentity,
}

impl MemberId {
    /// Creates a member ID from its declaring identity, category, position, and
    /// fragment.
    ///
    /// # Parameters
    ///
    /// - `declaring_identity`: Stable identity of the owning descriptor.
    /// - `kind`: Member category.
    /// - `index`: Declaration position within that category.
    /// - `fragment`: Registration fragment that contributed the member.
    ///
    /// # Returns
    ///
    /// Returns the composite member identity.
    #[must_use]
    pub fn new(declaring_identity: &str, kind: &str, index: usize, fragment: FragmentIdentity) -> Self {
        Self {
            declaring_identity: declaring_identity.into(),
            kind: kind.into(),
            index,
            fragment,
        }
    }

    /// Returns the identity of the descriptor declaring this member.
    ///
    /// # Returns
    ///
    /// Returns the stable identity of the owning descriptor.
    #[must_use]
    #[inline]
    pub fn declaring_identity(&self) -> &str {
        &self.declaring_identity
    }

    /// Returns the member category.
    ///
    /// # Returns
    ///
    /// Returns the member's category label.
    #[must_use]
    #[inline]
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// Returns the member's declaration index within its category.
    ///
    /// # Returns
    ///
    /// Returns the zero-based declaration index.
    #[must_use]
    #[inline]
    pub fn index(&self) -> usize {
        self.index
    }

    /// Returns the fragment that contributed this member.
    ///
    /// # Returns
    ///
    /// Returns the member's source and content identity.
    #[must_use]
    #[inline]
    pub fn fragment(&self) -> &FragmentIdentity {
        &self.fragment
    }
}
