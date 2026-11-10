// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::hint::black_box;

use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::criterion_group;
use criterion::criterion_main;
use qubit_reflect::__private::codegen_v4::expression::ExpressionData;
use qubit_reflect::__private::codegen_v4::expression::TypeExpressionData;
use qubit_reflect::__private::codegen_v4::expression::type_expression_in;
use qubit_reflect::ReflectionContext;
use qubit_reflect::descriptor::TypeExpression;
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
static U32: TypeExpressionData = ExpressionData::Named {
    path: "u32",
    definition: None,
    arguments: &[],
};
static U64: TypeExpressionData = ExpressionData::Named {
    path: "u64",
    definition: None,
    arguments: &[],
};
static EMPTY_ELEMENTS: [&TypeExpressionData; 0] = [];
static PAIR_ELEMENTS: [&TypeExpressionData; 2] = [&U8, &U16];
static WIDE_ELEMENTS: [&TypeExpressionData; 4] = [&U8, &U16, &U32, &U64];
static NESTED_ELEMENTS: [&TypeExpressionData; 2] = [&PAIR, &WIDE];
static EMPTY: TypeExpressionData = ExpressionData::Tuple(&EMPTY_ELEMENTS);
static PAIR: TypeExpressionData = ExpressionData::Tuple(&PAIR_ELEMENTS);
static WIDE: TypeExpressionData = ExpressionData::Tuple(&WIDE_ELEMENTS);
static NESTED: TypeExpressionData = ExpressionData::Tuple(&NESTED_ELEMENTS);

fn count_kind(expression: TypeExpression<'_>) -> usize {
    let expression = black_box(expression);
    match expression.kind() {
        TypeExpressionKind::Tuple(elements) => elements.iter().map(|element| count_kind(*element)).sum(),
        _ => 1,
    }
}

fn count_borrowed(expression: TypeExpression<'_>) -> usize {
    let expression = black_box(expression);
    match expression.tuple_elements() {
        Some(elements) => elements.iter().map(count_borrowed).sum(),
        None => 1,
    }
}

fn tuple_traversal(c: &mut Criterion) {
    let context = ReflectionContext::builder().build().expect("empty context builds");
    let mut group = c.benchmark_group("type_expression/tuple_kind_traversal");
    for (name, data) in [("empty", &EMPTY), ("pair", &PAIR), ("wide", &WIDE), ("nested", &NESTED)] {
        group.bench_function(BenchmarkId::new("kind", name), |b| {
            b.iter(|| {
                let expression = type_expression_in(data, &context);
                black_box(count_kind(expression))
            });
        });
        group.bench_function(BenchmarkId::new("tuple_elements", name), |b| {
            b.iter(|| {
                let expression = type_expression_in(data, &context);
                black_box(count_borrowed(expression))
            });
        });
    }
    group.finish();
}

criterion_group!(benches, tuple_traversal);
criterion_main!(benches);
