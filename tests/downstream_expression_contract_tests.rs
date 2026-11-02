// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::__private::codegen_v4::PROTOCOL_VERSION;
use qubit_reflect::__private::codegen_v4::expression::ExpressionData;
use qubit_reflect::__private::codegen_v4::expression::GenericExpressionArgument;
use qubit_reflect::__private::codegen_v4::expression::TypeExpressionData;
use qubit_reflect::__private::codegen_v4::expression::type_expression_in;
use qubit_reflect::__private::codegen_v4::protocol::assert_protocol_version;
use qubit_reflect::ReflectionContext;
use qubit_reflect::descriptor::GenericExpressionArgumentKind;
use qubit_reflect::descriptor::TypeExpressionKind;

static STRING: TypeExpressionData = ExpressionData::Named {
    path: "String",
    definition: None,
    arguments: &[],
};
static VEC_ARGUMENTS: [GenericExpressionArgument; 1] = [GenericExpressionArgument::Type(&STRING)];
static VECTOR: TypeExpressionData = ExpressionData::Named {
    path: "Vec",
    definition: None,
    arguments: &VEC_ARGUMENTS,
};
static OPTION_ARGUMENTS: [GenericExpressionArgument; 1] = [GenericExpressionArgument::Type(&VECTOR)];
static NESTED: TypeExpressionData = ExpressionData::Named {
    path: "Option",
    definition: None,
    arguments: &OPTION_ARGUMENTS,
};

#[test]
fn downstream_static_expression_views_keep_the_supplied_context() {
    assert_eq!(PROTOCOL_VERSION, 4);
    assert_protocol_version(4);
    let context = ReflectionContext::builder()
        .include_type::<Option<Vec<String>>>()
        .build()
        .expect("the explicit context builds");

    let expression = type_expression_in(&NESTED, &context);
    assert!(std::ptr::eq(expression.context(), &context));
    let TypeExpressionKind::Named(option) = expression.kind() else {
        panic!("the root expression is a named type");
    };
    let GenericExpressionArgumentKind::Type(vector_expression) = option
        .arguments()
        .get(0)
        .expect("Option has one generic argument")
        .kind()
    else {
        panic!("Option's argument is a type expression");
    };
    assert!(std::ptr::eq(vector_expression.context(), &context));
    let TypeExpressionKind::Named(vector) = vector_expression.kind() else {
        panic!("the nested expression is a named type");
    };
    let GenericExpressionArgumentKind::Type(string_expression) =
        vector.arguments().get(0).expect("Vec has one generic argument").kind()
    else {
        panic!("Vec's argument is a type expression");
    };
    assert!(std::ptr::eq(string_expression.context(), &context));
}
