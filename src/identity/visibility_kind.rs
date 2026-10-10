// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Categories used by normalized source visibility.

/// A normalized source visibility category.
///
/// # Examples
///
/// ```
/// use qubit_reflect::identity::VisibilityKind;
/// assert_eq!(VisibilityKind::Public, VisibilityKind::Public);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum VisibilityKind {
    /// Public outside the declaring crate.
    Public,
    /// Visible throughout the declaring crate.
    Crate,
    /// Visible from the parent module and its descendants.
    Super,
    /// Visible from an explicitly named source scope and its descendants.
    Restricted,
    /// Visible within the declaring module and its descendants.
    Private,
}
