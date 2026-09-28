// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! return kind definitions.

/// The structural category of a method return value.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::ReturnKind;
/// assert_eq!(ReturnKind::Unit, ReturnKind::Unit);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ReturnKind {
    /// The unit return type `()`.
    Unit,
    /// The never return type `!`.
    Never,
    /// A concrete owned value.
    Concrete,
    /// A shared or mutable reference.
    Reference,
    /// An opaque `impl Trait` return value.
    Opaque,
}
