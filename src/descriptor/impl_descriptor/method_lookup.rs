// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! MethodLookup metadata and behavior.

use crate::descriptor::MethodInstanceDescriptor;

/// The result of a method lookup across implementation namespaces.
///
/// # Type Parameters
///
/// - `'a`: Lifetime of a concrete method instance borrowed from an impl.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::MethodLookup;
/// let result = MethodLookup::Missing;
/// assert!(matches!(result, MethodLookup::Missing));
/// ```
#[derive(Clone, Copy, Debug)]
pub enum MethodLookup<'a> {
    /// No matching concrete method instance exists.
    Missing,
    /// Exactly one concrete method instance matches; the variant contains it.
    Unique(&'a MethodInstanceDescriptor),
    /// Multiple namespaces or fragments match the query.
    Ambiguous,
}
