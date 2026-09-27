// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Stable identities for distributed registration fragments.

use std::sync::Arc;

/// The source and content identity of one registration fragment.
///
/// # Examples
///
/// ```
/// use qubit_reflect::identity::FragmentIdentity;
/// let identity = FragmentIdentity::new("example", "demo", 10, 1, "type", 42);
/// assert_eq!(identity.declaring_crate(), "example");
/// ```
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FragmentIdentity {
    declaring_crate: Arc<str>,
    module_path: Arc<str>,
    line: u32,
    column: u32,
    member_kind: Arc<str>,
    content_fingerprint: u64,
}

impl FragmentIdentity {
    /// Creates a fragment identity from its stable source and content facts.
    ///
    /// # Parameters
    ///
    /// - `declaring_crate`: Crate containing the declaration.
    /// - `module_path`: Module path containing the declaration.
    /// - `line`: Declaration line in the source file.
    /// - `column`: Declaration column in the source file.
    /// - `member_kind`: Kind of registration member represented.
    /// - `content_fingerprint`: Deterministic fingerprint of normalized input.
    ///
    /// # Returns
    ///
    /// Returns the stable source and content identity.
    #[must_use]
    pub fn new(
        declaring_crate: &str,
        module_path: &str,
        line: u32,
        column: u32,
        member_kind: &str,
        content_fingerprint: u64,
    ) -> Self {
        Self {
            declaring_crate: declaring_crate.into(),
            module_path: module_path.into(),
            line,
            column,
            member_kind: member_kind.into(),
            content_fingerprint,
        }
    }

    /// Returns the crate that declared this fragment.
    ///
    /// # Returns
    ///
    /// Returns the declaring crate name.
    #[must_use]
    #[inline]
    pub fn declaring_crate(&self) -> &str {
        &self.declaring_crate
    }

    /// Returns the declaring module path.
    ///
    /// # Returns
    ///
    /// Returns the module path containing the declaration.
    #[must_use]
    #[inline]
    pub fn module_path(&self) -> &str {
        &self.module_path
    }

    /// Returns the declaration line number.
    ///
    /// # Returns
    ///
    /// Returns the source line number.
    #[must_use]
    #[inline]
    pub fn line(&self) -> u32 {
        self.line
    }

    /// Returns the declaration column number.
    ///
    /// # Returns
    ///
    /// Returns the source column number.
    #[must_use]
    #[inline]
    pub fn column(&self) -> u32 {
        self.column
    }

    /// Returns the category of members declared by this fragment.
    ///
    /// # Returns
    ///
    /// Returns the member category.
    #[must_use]
    #[inline]
    pub fn member_kind(&self) -> &str {
        &self.member_kind
    }

    /// Returns the deterministic fingerprint of normalized macro input.
    ///
    /// # Returns
    ///
    /// Returns the content fingerprint.
    #[must_use]
    #[inline]
    pub fn content_fingerprint(&self) -> u64 {
        self.content_fingerprint
    }

    /// Returns whether two identities share source coordinates and member kind.
    ///
    /// # Parameters
    ///
    /// - `other`: Identity whose source coordinates and member kind are
    ///   compared.
    ///
    /// # Returns
    ///
    /// Returns `true` when both identities refer to the same source
    /// declaration.
    pub(crate) fn same_source_identity(&self, other: &Self) -> bool {
        self.declaring_crate == other.declaring_crate
            && self.module_path == other.module_path
            && self.line == other.line
            && self.column == other.column
            && self.member_kind == other.member_kind
    }
}
