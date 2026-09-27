// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Modifiers attached to structural trait bounds.

/// The modifier attached to one structural trait bound.
///
/// # Examples
///
/// ```
/// use qubit_reflect::expression::TraitBoundModifier;
/// let modifier = TraitBoundModifier::Maybe;
/// assert_eq!(modifier, TraitBoundModifier::Maybe);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TraitBoundModifier {
    /// The trait bound is required.
    None,
    /// The trait bound uses Rust's `?Trait` relaxation syntax.
    Maybe,
}
