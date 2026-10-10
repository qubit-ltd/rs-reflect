// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Identifies the standard collection family used to represent a set.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::SetKind;
///
/// let kind = SetKind::BTreeSet;
/// assert!(matches!(kind, SetKind::BTreeSet));
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SetKind {
    /// Selects the hash-based `HashSet<T>` collection family.
    HashSet,
    /// Selects the tree-based `BTreeSet<T>` collection family.
    BTreeSet,
}
