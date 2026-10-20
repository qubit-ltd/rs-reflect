// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Static shape access stays independent from context discovery indexes.

#![cfg(feature = "derive")]

use qubit_reflect::Reflect;
use qubit_reflect::ReflectionContext;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::construct::ConstructionAvailability;
use qubit_reflect::construct::ConstructionError;
use qubit_reflect::construct::ConstructionShape;
use qubit_reflect::construct::ConstructionUnavailableReason;
use qubit_reflect::construct::NamedFields;
use qubit_reflect::construct::PositionalFields;
use qubit_reflect::value::Local;

#[Reflect]
struct ContextIndependentShape {
    value: u32,
}

#[Reflect]
struct ContextIndependentTuple(u32, u32);

#[Reflect]
struct ContextIndependentUnit;

#[Reflect]
struct ContextIndependentEmptyNamed {}

#[test]
fn type_metadata_remains_available_when_the_context_does_not_index_the_type() {
    let context = ReflectionContext::builder()
        .build()
        .expect("empty explicit contexts are valid");
    let descriptor = context.type_of::<ContextIndependentShape>();

    assert!(
        context
            .type_by_name(std::any::type_name::<ContextIndependentShape>())
            .is_err()
    );
    assert_eq!(descriptor.context().id(), context.id());
    assert_eq!(
        descriptor
            .field("value")
            .expect("static field metadata is available")
            .name(),
        "value"
    );
    assert_eq!(
        TypeDescriptor::of::<ContextIndependentShape>()
            .field("value")
            .expect("global descriptor also exposes static shape")
            .name(),
        "value"
    );
    assert!(context.types().expect("empty context stays valid").is_empty());
}

#[test]
fn static_construction_shapes_remain_available_without_context_registration() {
    let context = ReflectionContext::builder()
        .build()
        .expect("empty explicit contexts are valid");

    let named = context.type_of::<ContextIndependentShape>().construction();
    assert_eq!(named.shape(), ConstructionShape::Named);
    assert_eq!(
        named.availability(),
        ConstructionAvailability::Unavailable {
            reasons: Box::new([ConstructionUnavailableReason::NoProvider]),
        }
    );
    let named_failure =
        match named.construct::<ContextIndependentShape, _>(NamedFields::<Local>::new().field("value", 1_u32)) {
            Ok(_) => panic!("an empty context does not provide a construction adapter"),
            Err(failure) => failure,
        };
    assert!(matches!(
        named_failure.error(),
        ConstructionError::Unavailable(ConstructionAvailability::Unavailable { reasons })
            if reasons.as_ref() == [ConstructionUnavailableReason::NoProvider]
    ));

    let tuple = context.type_of::<ContextIndependentTuple>().construction();
    assert_eq!(tuple.shape(), ConstructionShape::Positional);
    assert!(matches!(
        tuple.construct::<ContextIndependentTuple, _>(
            PositionalFields::<Local>::new().value(1_u32).value(2_u32)
        ),
        Err(failure) if matches!(failure.error(), ConstructionError::Unavailable(_))
    ));
    assert_eq!(
        context.type_of::<ContextIndependentUnit>().construction().shape(),
        ConstructionShape::Unit,
    );
    assert_eq!(
        context.type_of::<ContextIndependentEmptyNamed>().construction().shape(),
        ConstructionShape::Named,
    );
}
