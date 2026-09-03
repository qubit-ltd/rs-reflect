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
use qubit_reflect::expression::GenericArgument;
use qubit_reflect::expression::TypeExpression;

const MAX_INPUT_BYTES: usize = 4_096;

// Exercises bounded public structural type-expression construction with
// arbitrary lossy UTF-8 text.
fuzz_target!(|unbounded: &[u8]| {
    let data = &unbounded[..unbounded.len().min(MAX_INPUT_BYTES)];
    let data = String::from_utf8_lossy(data).into_owned();
    let Ok(parameter) = TypeExpression::parameter(data.to_string()) else {
        return;
    };
    let Ok(concrete) = ConcreteTypeExpression::new([data.into_boxed_str()], [GenericArgument::Type(parameter)]) else {
        return;
    };
    let expression = TypeExpression::Concrete(concrete);
    assert_eq!(expression, expression.clone());
    let _ = format!("{expression:?}");
});
