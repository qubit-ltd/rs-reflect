// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// A standard set family.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::SetKind;
/// assert_eq!(SetKind::HashSet, SetKind::HashSet);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SetKind {
    /// `HashSet<T>`.
    HashSet,
    /// `BTreeSet<T>`.
    BTreeSet,
}
