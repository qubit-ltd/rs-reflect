// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Trait identity hints accepted by method binding macros.

/// Identifies the source used to resolve a trait method's metadata.
pub(super) enum TraitBindingHint {
    /// A trait declared and reflected in the current crate.
    Reflected(
        /// Qualified path to the locally reflected trait.
        syn::Path,
    ),
    /// A trait implementation whose metadata is supplied externally.
    External(
        /// Path to the external trait marker type.
        syn::Path,
    ),
}
