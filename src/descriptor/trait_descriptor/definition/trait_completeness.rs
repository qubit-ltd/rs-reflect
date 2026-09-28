// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Completeness classification for reflected trait declarations.

/// How much of a trait declaration is known to reflection.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::TraitCompleteness;
///
/// assert_ne!(TraitCompleteness::Complete, TraitCompleteness::ExternalIncomplete);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TraitCompleteness {
    /// The trait declaration, supertraits, and associated items are known.
    Complete,
    /// Only facts proven by an observed external trait impl are known.
    ExternalIncomplete,
}
