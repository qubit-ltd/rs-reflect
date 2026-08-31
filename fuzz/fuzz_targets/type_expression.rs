// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![no_main]

use libfuzzer_sys::fuzz_target;
use qubit_reflect::expression::ConcreteTypeExpression;
use qubit_reflect::expression::DiagnosticText;
use qubit_reflect::expression::GenericArgument;
use qubit_reflect::expression::TypeExpression;

const MAX_INPUT_CHARS: usize = 4_096;

// Exercises bounded public structural type-expression construction with
// arbitrary text.
fuzz_target!(|data: String| {
    let data = data.chars().take(MAX_INPUT_CHARS).collect::<String>();
    let expression = TypeExpression::Concrete(ConcreteTypeExpression {
        path: Box::new([data.clone().into_boxed_str()]),
        arguments: Box::new([GenericArgument::Type(TypeExpression::Parameter(data.into_boxed_str()))]),
        diagnostic: DiagnosticText::default(),
    });
    assert_eq!(expression, expression.clone());
    let _ = format!("{expression:?}");
});
