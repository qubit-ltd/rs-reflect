// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::__private::codegen_v4::expression::ExpressionData;
use qubit_reflect::__private::codegen_v4::expression::TypeExpressionData;
use qubit_reflect::__private::codegen_v4::expression::type_expression_in;
use qubit_reflect::ReflectionContext;
use qubit_reflect::descriptor::TypeExpressionKind;

static U8: TypeExpressionData = ExpressionData::Named {
    path: "u8",
    definition: None,
    arguments: &[],
};
static U16: TypeExpressionData = ExpressionData::Named {
    path: "u16",
    definition: None,
    arguments: &[],
};
static PAIR_ELEMENTS: [&TypeExpressionData; 2] = [&U8, &U16];
static EMPTY_ELEMENTS: [&TypeExpressionData; 0] = [];
static PAIR: TypeExpressionData = ExpressionData::Tuple(&PAIR_ELEMENTS);
static EMPTY: TypeExpressionData = ExpressionData::Tuple(&EMPTY_ELEMENTS);
static NESTED: TypeExpressionData = ExpressionData::Tuple(&[&PAIR]);

#[test]
fn test_tuple_expression_view_keeps_order_context_and_legacy_kind_compatibility() {
    let context = ReflectionContext::builder().build().expect("empty context builds");
    let expression = type_expression_in(&PAIR, &context);
    let view = expression.tuple_elements().expect("tuple gives a view");
    assert_eq!(view.len(), 2);
    assert!(!view.is_empty());
    assert!(view.get(2).is_none());
    assert_eq!(view.iter().count(), 2);
    let viewed = view
        .iter()
        .map(|element| element.display().to_string())
        .collect::<Vec<_>>();
    let TypeExpressionKind::Tuple(legacy) = expression.kind() else {
        panic!("legacy kind remains a tuple");
    };
    assert_eq!(
        viewed,
        legacy
            .iter()
            .map(|element| element.display().to_string())
            .collect::<Vec<_>>()
    );
    assert_eq!(viewed, ["u8", "u16"]);
    assert!(view.iter().all(|element| std::ptr::eq(element.context(), &context)));
}

#[test]
fn test_tuple_expression_view_handles_empty_non_tuple_and_nested_elements() {
    let context = ReflectionContext::builder().build().expect("empty context builds");
    let empty = type_expression_in(&EMPTY, &context)
        .tuple_elements()
        .expect("unit tuple is Some");
    assert!(empty.is_empty());
    assert_eq!(empty.len(), 0);
    assert_eq!(empty.iter().count(), 0);
    let non_tuple = type_expression_in(&U8, &context);
    assert!(non_tuple.tuple_elements().is_none());
    let nested = type_expression_in(&NESTED, &context).tuple_elements().unwrap();
    let child = nested.get(0).expect("nested child exists");
    assert!(matches!(child.kind(), TypeExpressionKind::Tuple(_)));
    assert!(std::ptr::eq(child.context(), &context));
}
