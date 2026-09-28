// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! StructKind category metadata.

/// The declared shape of a Rust struct.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::StructKind;
/// let shape = StructKind::Newtype;
/// assert_eq!(shape, StructKind::Newtype);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum StructKind {
    /// A struct with named fields.
    Named,
    /// A struct with two or more positional fields.
    Tuple,
    /// A tuple struct with exactly one field.
    Newtype,
    /// A struct with no fields.
    Unit,
}
