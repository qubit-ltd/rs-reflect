// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Mutability category metadata.

/// The mutability of a raw pointer.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::Mutability;
/// let mutability = Mutability::Const;
/// assert_eq!(mutability, Mutability::Const);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Mutability {
    /// A const raw pointer.
    Const,
    /// A mutable raw pointer.
    Mutable,
}
