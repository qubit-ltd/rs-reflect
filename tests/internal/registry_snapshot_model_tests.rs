// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Shared public-API invariants for deterministic context-model seeds.

use std::any::TypeId;

use qubit_reflect::ReflectionContext;
use qubit_reflect::context::ReflectionContextBuilder;
use qubit_reflect::error::ContextError;

const MAX_INPUT_BYTES: usize = 4_096;
const MAX_TYPES: usize = 32;

fn include_type(builder: ReflectionContextBuilder, index: u8) -> ReflectionContextBuilder {
    match index % 4 {
        0 => builder.include_type::<u8>(),
        1 => builder.include_type::<u16>(),
        2 => builder.include_type::<u32>(),
        _ => builder.include_type::<u64>(),
    }
}

fn build<'a>(operations: impl Iterator<Item = &'a u8>) -> Result<ReflectionContext, ContextError> {
    let builder = operations
        .take(MAX_TYPES)
        .fold(ReflectionContext::builder(), |builder, operation| {
            include_type(builder, *operation)
        });
    builder.build()
}

fn contains(context: &ReflectionContext, id: TypeId) -> bool {
    context
        .find_type_by_id(id)
        .expect("a frozen context can answer type-identity lookups")
        .is_some()
}

fn observations(context: &ReflectionContext) -> [bool; 4] {
    [
        contains(context, TypeId::of::<u8>()),
        contains(context, TypeId::of::<u16>()),
        contains(context, TypeId::of::<u32>()),
        contains(context, TypeId::of::<u64>()),
    ]
}

/// Checks insertion-order invariance and explicit-context isolation.
///
/// # Parameters
///
/// - `unbounded`: Arbitrary input bytes, limited internally to 4 KiB and 32
///   type registrations.
///
/// # Panics
///
/// Panics if context validity or indexed types depend on registration order, or
/// if one explicit context changes another context's frozen type set.
pub fn check(unbounded: &[u8]) {
    let data = &unbounded[..unbounded.len().min(MAX_INPUT_BYTES)];
    let first = build(data.iter());
    let reversed = build(data.iter().rev());
    match (first, reversed) {
        (Ok(first), Ok(reversed)) => assert_eq!(
            observations(&first),
            observations(&reversed),
            "context indexes must not depend on registration order"
        ),
        (Err(first), Err(reversed)) => {
            assert!(
                !first.issues().is_empty(),
                "the forward context error must identify its validation issue"
            );
            assert!(
                !reversed.issues().is_empty(),
                "the reversed context error must identify its validation issue"
            );
        }
        _ => panic!("context validity must not depend on registration order"),
    }

    let first = ReflectionContext::builder()
        .include_type::<u32>()
        .build()
        .expect("a single builtin registration is valid");
    let second = ReflectionContext::builder()
        .include_type::<u64>()
        .build()
        .expect("another isolated builtin registration is valid");
    assert_eq!(observations(&first), [false, false, true, false]);
    assert_eq!(observations(&second), [false, false, false, true]);
}
