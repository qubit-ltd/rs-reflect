// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![cfg(feature = "derive")]
#![allow(dead_code, non_camel_case_types, non_upper_case_globals)]

use std::any::TypeId;

use qubit_reflect::Reflect;
use qubit_reflect::ReflectionContext;
use qubit_reflect::TypeDescriptor;

#[Reflect]
struct Record<r#type, const r#const: usize> {
    value: r#type,
    bytes: [u8; r#const],
}

#[Reflect]
enum Choice<r#type, const r#const: usize> {
    Value(r#type),
    Bytes([u8; r#const]),
}

#[test]
fn test_raw_generic_names_preserve_definition_and_argument_metadata() {
    let context = ReflectionContext::builder()
        .include_type::<Record<u32, 2>>()
        .include_type::<Choice<u32, 3>>()
        .build()
        .expect("reflection context builds");

    let record = TypeDescriptor::of_in::<Record<u32, 2>>(&context);
    assert_eq!(
        record.find_definition().expect("record definition exists").name(),
        "Record"
    );
    let record_arguments = record.generic_arguments();
    assert_eq!(record_arguments.iter().len(), 2);
    assert_eq!(
        record_arguments
            .argument("r#type")
            .expect("raw type argument exists")
            .type_argument()
            .expect("argument is a type")
            .concrete_type()
            .expect("type argument has an exact type")
            .type_id(),
        TypeId::of::<u32>()
    );
    assert_eq!(
        record_arguments
            .argument("r#const")
            .expect("raw const argument exists")
            .const_argument()
            .expect("argument is const")
            .expression(),
        qubit_reflect::descriptor::ConstExpression::UnsignedInteger(2)
    );

    let choice = TypeDescriptor::of_in::<Choice<u32, 3>>(&context);
    assert_eq!(
        choice.find_definition().expect("choice definition exists").name(),
        "Choice"
    );
    let choice_arguments = choice.generic_arguments();
    assert_eq!(choice_arguments.iter().len(), 2);
    assert_eq!(
        choice_arguments
            .argument("r#type")
            .expect("raw type argument exists")
            .type_argument()
            .expect("argument is a type")
            .concrete_type()
            .expect("type argument has an exact type")
            .type_id(),
        TypeId::of::<u32>()
    );
    assert_eq!(
        choice_arguments
            .argument("r#const")
            .expect("raw const argument exists")
            .const_argument()
            .expect("argument is const")
            .expression(),
        qubit_reflect::descriptor::ConstExpression::UnsignedInteger(3)
    );
}
