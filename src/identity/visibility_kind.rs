// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Categories used by normalized source visibility.

/// A normalized source visibility category.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum VisibilityKind {
    /// Public outside the declaring crate.
    Public,
    /// Visible throughout the declaring crate.
    Crate,
    /// Visible to the immediate parent module.
    Super,
    /// Visible only in an explicitly named source scope.
    Restricted,
    /// Visible only in the declaring module.
    Private,
}
