// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! ReferenceKind category metadata.

/// The borrowing mode of a reference.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::ReferenceKind;
/// let reference = ReferenceKind::Shared;
/// assert_eq!(reference, ReferenceKind::Shared);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ReferenceKind {
    /// A shared reference.
    Shared,
    /// An exclusive mutable reference.
    Mutable,
}
