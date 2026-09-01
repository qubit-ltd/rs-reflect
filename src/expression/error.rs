// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Errors reported while constructing structural expressions.

/// An invariant violation in a structural expression.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum ExpressionError {
    /// A concrete type path did not contain any segments.
    #[error("a concrete type path must contain at least one segment")]
    EmptyConcretePath,
    /// Trait bounds and their modifiers had different lengths.
    #[error("trait-bound modifiers must match bounds: {bounds} bounds, {modifiers} modifiers")]
    BoundModifierCount {
        /// Number of trait bounds.
        bounds: usize,
        /// Number of corresponding modifiers.
        modifiers: usize,
    },
}
