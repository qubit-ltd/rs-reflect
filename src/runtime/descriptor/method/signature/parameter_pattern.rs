// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Source pattern retained for a non-receiver parameter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParameterPattern {
    /// A parameter bound to the given identifier.
    Identifier(
        /// Identifier text retained from the source pattern.
        &'static str,
    ),
    /// A wildcard parameter with no binding name.
    Wildcard,
    /// A destructured parameter whose nested pattern is not retained.
    Destructure,
    /// A destructured parameter with its source pattern text.
    DestructurePattern(
        /// Original pattern text, retained for diagnostics and inspection.
        &'static str,
    ),
}
