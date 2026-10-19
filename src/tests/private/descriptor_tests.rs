// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0 (the "License");
//    you may not use this file except in compliance with the License.
// =============================================================================

//! Unit checks for the versioned v4 expression metadata consumed by derives.

use crate::__private::codegen_v4::expression::{
    ConstExpression, ExpressionData, FunctionPointerData, GenericExpressionArgument,
    LifetimeExpression, TraitExpressionData,
};

static U8: ExpressionData = ExpressionData::Named {
    path: "u8",
    definition: None,
    arguments: &[],
};

static TYPE_ARGUMENTS: [GenericExpressionArgument; 1] =
    [GenericExpressionArgument::Type(&U8)];
static VEC_U8: ExpressionData = ExpressionData::Named {
    path: "std::vec::Vec",
    definition: None,
    arguments: &TYPE_ARGUMENTS,
};
static ARRAY: ExpressionData = ExpressionData::Array {
    element: &U8,
    length: ConstExpression::Parameter("N"),
};
static TUPLE_ITEMS: [&ExpressionData; 2] = [&U8, &VEC_U8];
static TUPLE: ExpressionData = ExpressionData::Tuple(&TUPLE_ITEMS);
static FUNCTION_ARGUMENTS: [&ExpressionData; 1] = [&ARRAY];
static FUNCTION: FunctionPointerData = FunctionPointerData::new(
    &FUNCTION_ARGUMENTS,
    Some(&TUPLE),
    true,
    Some("C"),
    false,
    &[LifetimeExpression::Named("a")],
);
static FUNCTION_EXPRESSION: ExpressionData = ExpressionData::FunctionPointer(&FUNCTION);
static TRAIT_ARGUMENTS: [GenericExpressionArgument; 1] =
    [GenericExpressionArgument::Const(ConstExpression::UnsignedInteger(8))];
static TRAIT_EXPRESSION: TraitExpressionData =
    TraitExpressionData::new("fixture::Bound", &TRAIT_ARGUMENTS, &[]);

/// Verifies generated metadata records nested type and const expressions
/// structurally instead of flattening them to diagnostic strings.
#[test]
fn test_codegen_v4_expression_metadata_preserves_structural_inputs() {
    let ExpressionData::Named { path, arguments, .. } = VEC_U8 else {
        panic!("the vector expression remains a named path");
    };
    assert_eq!(path, "std::vec::Vec");
    let GenericExpressionArgument::Type(argument) = arguments[0] else {
        panic!("the vector argument remains a type expression");
    };
    assert!(std::ptr::eq(argument, &U8));

    let ExpressionData::Array { element, length } = ARRAY else {
        panic!("the array expression retains its array structure");
    };
    assert!(std::ptr::eq(element, &U8));
    assert_eq!(length, ConstExpression::Parameter("N"));

    let ExpressionData::Tuple(items) = TUPLE else {
        panic!("the tuple expression retains ordered elements");
    };
    assert_eq!(items.len(), 2);
    assert!(std::ptr::eq(items[1], &VEC_U8));

    let ExpressionData::FunctionPointer(function) = FUNCTION_EXPRESSION else {
        panic!("the function expression retains signature metadata");
    };
    assert!(function.unsafe_);
    assert_eq!(function.abi, Some("C"));
    assert_eq!(function.lifetimes, &[LifetimeExpression::Named("a")]);
    assert!(std::ptr::eq(function.arguments[0], &ARRAY));
    assert!(std::ptr::eq(function.result.unwrap(), &TUPLE));

    assert_eq!(TRAIT_EXPRESSION.path, "fixture::Bound");
    assert!(matches!(
        TRAIT_EXPRESSION.arguments[0],
        GenericExpressionArgument::Const(ConstExpression::UnsignedInteger(8))
    ));
}
