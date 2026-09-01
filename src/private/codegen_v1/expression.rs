// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Checked expression factories consumed by generated code.

use crate::expression::ConcreteTypeExpression;
use crate::expression::DiagnosticText;
use crate::expression::GenericArgument;

/// Creates a concrete type expression from generator-validated path data.
#[doc(hidden)]
pub fn concrete(
    path: Box<[Box<str>]>,
    arguments: Box<[GenericArgument]>,
    diagnostic: DiagnosticText,
) -> ConcreteTypeExpression {
    let mut expression = ConcreteTypeExpression::new(path, arguments).expect("generated concrete paths are non-empty");
    expression.diagnostic = diagnostic;
    expression
}
