// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Syntax-parser output retained until validation completes.

use syn::Error;

use crate::ir::ParsedDeclaration;

/// Holds parser diagnostics alongside IR until the validation pipeline
/// completes.
pub(super) struct ParsedPipeline {
    /// The declaration produced by syntax parsing.
    pub(super) declaration: ParsedDeclaration,
    /// Recoverable syntax diagnostics collected during parsing.
    pub(super) error: Option<Error>,
}
