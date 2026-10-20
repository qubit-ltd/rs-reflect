// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![no_main]

use libfuzzer_sys::fuzz_target;
use qubit_reflect::Reflect;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::descriptor::TypeExpressionKind;

const MAX_INPUT_BYTES: usize = 4_096;

#[Reflect]
struct ExpressionCorpus {
    nested: Vec<Option<(u8, [u16; 3])>>,
    reference: &'static [u8],
    pointer: *const u32,
    tuple: (u8, Vec<&'static str>),
    function: fn(u16) -> usize,
}

fn walk_expression(expression: qubit_reflect::descriptor::TypeExpression<'_>, data: &[u8], offset: &mut usize) {
    let _ = expression.display().to_string();
    let selection = data.get(*offset).copied().unwrap_or_default() as usize;
    *offset = offset.saturating_add(1);

    match expression.kind() {
        TypeExpressionKind::Named(named) => {
            let _ = named.path();
            let _ = named.find_definition();
            let arguments = named.arguments();
            if !arguments.is_empty() {
                let argument = arguments
                    .get(selection % arguments.len())
                    .expect("selected argument exists");
                if let qubit_reflect::descriptor::GenericExpressionArgumentKind::Type(ty) = argument.kind() {
                    walk_expression(ty, data, offset);
                }
            }
        }
        TypeExpressionKind::Reference { target, .. }
        | TypeExpressionKind::RawPointer { target, .. }
        | TypeExpressionKind::Slice(target) => walk_expression(target, data, offset),
        TypeExpressionKind::Array { element, .. } => walk_expression(element, data, offset),
        TypeExpressionKind::Tuple(elements) if !elements.is_empty() => {
            walk_expression(elements[selection % elements.len()], data, offset);
        }
        TypeExpressionKind::AssociatedType(associated) => {
            let _ = associated.name();
            if let Some(trait_) = associated.trait_() {
                let _ = trait_.path();
                let _ = trait_.find_definition();
            }
            walk_expression(associated.self_type(), data, offset);
        }
        TypeExpressionKind::FunctionPointer(function) => {
            let arguments = function.arguments();
            if !arguments.is_empty() {
                walk_expression(arguments[selection % arguments.len()], data, offset);
            } else if let Some(result) = function.result() {
                walk_expression(result, data, offset);
            }
            let _ = function.abi();
            let _ = function.is_unsafe();
            let _ = function.is_variadic();
        }
        TypeExpressionKind::TraitObject(object) => {
            for trait_ in object.traits() {
                let _ = trait_.path();
                let _ = trait_.find_definition();
                let _ = trait_.associated_type_constraints().len();
            }
        }
        TypeExpressionKind::Opaque(opaque) => {
            let _ = opaque.bounds().len();
        }
        TypeExpressionKind::Parameter(_)
        | TypeExpressionKind::ParameterName(_)
        | TypeExpressionKind::SelfType
        | TypeExpressionKind::Never
        | TypeExpressionKind::Tuple(_) => {}
    }
}

fuzz_target!(|unbounded: &[u8]| {
    let data = &unbounded[..unbounded.len().min(MAX_INPUT_BYTES)];
    let descriptor = TypeDescriptor::of::<ExpressionCorpus>();
    let fields = descriptor.fields();
    assert!(!fields.is_empty(), "the expression corpus must expose fields");

    // Input bytes select both a corpus root and a child at each nested node,
    // so mutations explore different expression shapes rather than only
    // repeating the same precompiled Vec<u8> expression.
    let mut offset = 0;
    let mut field_index = data.first().copied().unwrap_or_default() as usize % fields.len();
    for _ in 0..data.len().max(1).min(fields.len()) {
        let Some(field) = fields.get_at(field_index) else {
            break;
        };
        walk_expression(field.type_expression(), data, &mut offset);
        field_index = data.get(offset).copied().unwrap_or_default() as usize % fields.len();
        offset = offset.saturating_add(1);
    }
});
