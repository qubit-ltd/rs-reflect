// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Shared finite source-expression corpus for deterministic tests and fuzzing.

use qubit_reflect::__private::codegen_v4::expression::ExpressionData;
use qubit_reflect::__private::codegen_v4::expression::FunctionPointerData;
use qubit_reflect::__private::codegen_v4::expression::OpaqueTypeData;
use qubit_reflect::__private::codegen_v4::expression::PredicateData;
use qubit_reflect::__private::codegen_v4::expression::TraitBoundModifier;
use qubit_reflect::__private::codegen_v4::expression::TraitExpressionData;
use qubit_reflect::__private::codegen_v4::expression::TraitObjectData;
use qubit_reflect::__private::codegen_v4::expression::type_expression_in;
use qubit_reflect::Reflect;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::descriptor::TypeExpressionKind;

const MAX_INPUT_BYTES: usize = 4_096;
const DERIVED_ROOTS: usize = 5;

#[Reflect]
struct ExpressionCorpus {
    nested: Vec<Option<(u8, [u16; 3])>>,
    reference: &'static [u8],
    pointer: *const u32,
    tuple: (u8, Vec<&'static str>),
    function: fn(u16) -> usize,
}

static U16: ExpressionData = ExpressionData::Named {
    path: "u16",
    definition: None,
    arguments: &[],
};
static USIZE: ExpressionData = ExpressionData::Named {
    path: "usize",
    definition: None,
    arguments: &[],
};
static ASSOCIATED: ExpressionData = ExpressionData::AssociatedType {
    self_type: &U16,
    trait_: None,
    name: "Item",
    arguments: &[],
};
static TRAIT: TraitExpressionData = TraitExpressionData::new("core::fmt::Debug", &[], &[]);
static TRAITS: [TraitExpressionData; 1] = [TRAIT];
static OBJECT_DATA: TraitObjectData = TraitObjectData::new(&TRAITS, None);
static OBJECT: ExpressionData = ExpressionData::TraitObject(&OBJECT_DATA);
static PREDICATES: [PredicateData; 1] = [PredicateData::TraitBound {
    ty: &U16,
    trait_: &TRAIT,
    modifier: TraitBoundModifier::None,
}];
static OPAQUE_DATA: OpaqueTypeData = OpaqueTypeData::new(&PREDICATES);
static OPAQUE: ExpressionData = ExpressionData::Opaque(&OPAQUE_DATA);
static FUNCTION: FunctionPointerData = FunctionPointerData::new(&[&U16], Some(&USIZE), false, None, false, &[]);
static FUNCTION_EXPRESSION: ExpressionData = ExpressionData::FunctionPointer(&FUNCTION);
static FUNCTION_NO_ARGS: FunctionPointerData = FunctionPointerData::new(&[], Some(&USIZE), false, None, false, &[]);
static FUNCTION_NO_ARGS_EXPRESSION: ExpressionData = ExpressionData::FunctionPointer(&FUNCTION_NO_ARGS);
static FUNCTION_NO_RESULT: FunctionPointerData = FunctionPointerData::new(&[&U16], None, false, None, false, &[]);
static FUNCTION_NO_RESULT_EXPRESSION: ExpressionData = ExpressionData::FunctionPointer(&FUNCTION_NO_RESULT);
static FUNCTION_EMPTY: FunctionPointerData = FunctionPointerData::new(&[], None, false, None, false, &[]);
static FUNCTION_EMPTY_EXPRESSION: ExpressionData = ExpressionData::FunctionPointer(&FUNCTION_EMPTY);

/// Observations made only when a branch or edge was actually visited.
#[derive(Default, Debug)]
pub struct VisitReport {
    pub associated_types: usize,
    pub trait_objects: usize,
    pub opaque_types: usize,
    pub function_parameter_edges: usize,
    pub function_result_edges: usize,
    pub visited_u16: bool,
    pub visited_usize: bool,
}

fn root(
    index: usize,
    context: &qubit_reflect::ReflectionContext,
) -> Option<qubit_reflect::descriptor::TypeExpression<'_>> {
    if index < DERIVED_ROOTS {
        return TypeDescriptor::of::<ExpressionCorpus>()
            .fields()
            .get_at(index)
            .map(|field| field.type_expression());
    }
    let data = match index - DERIVED_ROOTS {
        0 => &ASSOCIATED,
        1 => &OBJECT,
        2 => &OPAQUE,
        3 => &FUNCTION_EXPRESSION,
        4 => &FUNCTION_NO_ARGS_EXPRESSION,
        5 => &FUNCTION_NO_RESULT_EXPRESSION,
        6 => &FUNCTION_EMPTY_EXPRESSION,
        _ => return None,
    };
    Some(type_expression_in(data, context))
}

fn walk(
    expression: qubit_reflect::descriptor::TypeExpression<'_>,
    data: &[u8],
    offset: &mut usize,
    report: &mut VisitReport,
) {
    let _ = expression.display().to_string();
    let selection = data.get(*offset).copied().unwrap_or_default() as usize;
    *offset = offset.saturating_add(1);
    match expression.kind() {
        TypeExpressionKind::Named(named) => {
            report.visited_u16 |= named.path() == "u16";
            report.visited_usize |= named.path() == "usize";
            let _ = named.find_definition();
            let arguments = named.arguments();
            if !arguments.is_empty()
                && let qubit_reflect::descriptor::GenericExpressionArgumentKind::Type(ty) = arguments
                    .get(selection % arguments.len())
                    .expect("selected argument exists")
                    .kind()
            {
                walk(ty, data, offset, report);
            }
        }
        TypeExpressionKind::Reference { target, .. }
        | TypeExpressionKind::RawPointer { target, .. }
        | TypeExpressionKind::Slice(target) => walk(target, data, offset, report),
        TypeExpressionKind::Array { element, .. } => walk(element, data, offset, report),
        TypeExpressionKind::Tuple(elements) if !elements.is_empty() => {
            walk(elements[selection % elements.len()], data, offset, report);
        }
        TypeExpressionKind::AssociatedType(associated) => {
            report.associated_types += 1;
            let _ = associated.name();
            if let Some(trait_) = associated.trait_() {
                let _ = trait_.path();
                let _ = trait_.find_definition();
            }
            walk(associated.self_type(), data, offset, report);
        }
        TypeExpressionKind::FunctionPointer(function) => {
            let arguments = function.arguments();
            let result = function.result();
            let child_count = arguments.len() + usize::from(result.is_some());
            if child_count != 0 {
                let child = selection % child_count;
                if child < arguments.len() {
                    report.function_parameter_edges += 1;
                    walk(arguments[child], data, offset, report);
                } else if let Some(result) = result {
                    report.function_result_edges += 1;
                    walk(result, data, offset, report);
                }
            }
            let _ = function.abi();
            let _ = function.is_unsafe();
            let _ = function.is_variadic();
        }
        TypeExpressionKind::TraitObject(object) => {
            report.trait_objects += 1;
            for trait_ in object.traits() {
                let _ = trait_.path();
                let _ = trait_.find_definition();
                let _ = trait_.associated_type_constraints().len();
            }
        }
        TypeExpressionKind::Opaque(opaque) => {
            report.opaque_types += 1;
            let _ = opaque.bounds().len();
        }
        TypeExpressionKind::Parameter(_)
        | TypeExpressionKind::ParameterName(_)
        | TypeExpressionKind::SelfType
        | TypeExpressionKind::Never
        | TypeExpressionKind::Tuple(_) => {}
    }
}

/// Visits a bounded root and its selected child path, returning actual visits.
pub fn check(unbounded: &[u8]) -> VisitReport {
    let data = &unbounded[..unbounded.len().min(MAX_INPUT_BYTES)];
    let context = qubit_reflect::ReflectionContext::builder()
        .build()
        .expect("an empty reflection context is valid");
    let root_count = DERIVED_ROOTS + 7;
    let root_index = data.first().copied().unwrap_or_default() as usize % root_count;
    let Some(expression) = root(root_index, &context) else {
        unreachable!("root selection stays within the finite corpus");
    };
    let mut report = VisitReport::default();
    let mut offset = 1;
    walk(expression, data, &mut offset, &mut report);
    report
}
