// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// A standard map family.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::MapKind;
/// assert_eq!(MapKind::BTreeMap, MapKind::BTreeMap);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MapKind {
    /// `HashMap<K, V>`.
    HashMap,
    /// `BTreeMap<K, V>`.
    BTreeMap,
}
