// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Source origin of enum discriminants.

/// Whether a variant's discriminant was written explicitly in Rust source.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::DiscriminantOrigin;
/// assert_eq!(DiscriminantOrigin::Implicit, DiscriminantOrigin::Implicit);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DiscriminantOrigin {
    /// Rust assigned the value from declaration order and preceding values.
    Implicit,
    /// The variant declaration contains an explicit discriminant expression.
    Explicit,
}
