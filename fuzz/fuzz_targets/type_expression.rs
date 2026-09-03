// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![no_main]

use libfuzzer_sys::fuzz_target;
use qubit_reflect::expression::ConcretePathSegment;
use qubit_reflect::expression::ConcreteTypeExpression;
use qubit_reflect::expression::ConstExpression;
use qubit_reflect::expression::ConstGenericArgument;
use qubit_reflect::expression::GenericArgument;
use qubit_reflect::expression::TypeExpression;

const MAX_INPUT_BYTES: usize = 4_096;

// Builds both compatibility and fully structural forms, then verifies the
// redundant compatibility projections cannot drift from structural identity.
fuzz_target!(|unbounded: &[u8]| {
    let data = &unbounded[..unbounded.len().min(MAX_INPUT_BYTES)];
    let data = String::from_utf8_lossy(data).into_owned();
    let Ok(parameter) = TypeExpression::parameter(data.to_string()) else {
        return;
    };
    let Ok(concrete) = ConcreteTypeExpression::new([data.clone().into_boxed_str()], [GenericArgument::Type(parameter)])
    else {
        return;
    };
    let expression = TypeExpression::Concrete(concrete);
    assert_eq!(expression, expression.clone());
    let _ = format!("{expression:?}");

    let segment_name = if data.is_empty() { "Item" } else { data.as_str() };
    let Ok(const_path) = ConstExpression::path(["limits", "MAX"]) else {
        unreachable!("fixed path is valid");
    };
    let u8_type = TypeExpression::Concrete(ConcreteTypeExpression::new(["u8"], []).expect("fixed concrete type"));
    let const_argument = GenericArgument::Const(ConstGenericArgument::new(u8_type, const_path, "limits::MAX"));
    let segments = [
        ConcretePathSegment::new("module", Box::default()),
        ConcretePathSegment::new(segment_name, vec![const_argument].into_boxed_slice()),
    ];
    let Ok(structural) = ConcreteTypeExpression::from_segments(segments) else {
        return;
    };
    assert_eq!(
        structural.path().iter().map(AsRef::as_ref).collect::<Vec<_>>(),
        structural
            .segments()
            .iter()
            .map(ConcretePathSegment::name)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        structural.arguments(),
        structural.segments().last().unwrap().arguments()
    );
    assert_eq!(structural, structural.clone().with_diagnostic("ignored"));
});
