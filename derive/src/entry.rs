// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Shared procedural-macro entry pipeline.

use proc_macro::TokenStream;

use crate::expand::dispatch;
use crate::ir::MacroKind;
use crate::parse::parse_and_validate_declaration;

/// Parses, validates, and expands one reflection macro invocation.
///
/// # Parameters
///
/// - `kind`: Reflection macro kind being invoked.
/// - `args`: Attribute arguments supplied to the macro.
/// - `input`: Rust item token stream passed to the macro.
///
/// # Returns
///
/// Returns expanded tokens or compiler-error tokens for the first pipeline
/// error.
#[must_use]
pub(crate) fn process_macro(kind: MacroKind, args: TokenStream, input: TokenStream) -> TokenStream {
    match parse_and_validate_declaration(kind, args.into(), input.into()) {
        Ok(validated) => match dispatch(validated.declaration) {
            Ok(expanded) => expanded.into(),
            Err(error) => error.into_compile_error().into(),
        },
        Err(error) => error.into_compile_error().into(),
    }
}
