// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Errors reported while constructing structural expressions.

// qubit-style: allow type-file-name

/// An invariant violation in a structural expression.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum ExpressionError {
    /// A concrete type path did not contain any segments.
    #[error("a concrete type path must contain at least one segment")]
    EmptyConcretePath,
    /// A type-bound predicate did not contain any trait bounds.
    #[error("a type-bound predicate must contain at least one bound")]
    EmptyTypeBounds,
    /// A lifetime-outlives predicate did not contain any bound lifetimes.
    #[error("a lifetime-outlives predicate must contain at least one bound")]
    EmptyLifetimeBounds,
    /// Trait bounds and their modifiers had different lengths.
    #[error("trait-bound modifiers must match bounds: {bounds} bounds, {modifiers} modifiers")]
    BoundModifierCount {
        /// Number of trait bounds.
        bounds: usize,
        /// Number of corresponding modifiers.
        modifiers: usize,
    },
}
