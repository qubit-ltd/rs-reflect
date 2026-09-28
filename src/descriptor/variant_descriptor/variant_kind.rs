// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Structural shapes of reflected enum variants.

/// The declared shape of an enum variant.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::VariantKind;
/// assert_eq!(VariantKind::Unit, VariantKind::Unit);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum VariantKind {
    /// A fieldless variant.
    Unit,
    /// A positional variant.
    Tuple,
    /// A variant with named fields.
    Struct,
}
