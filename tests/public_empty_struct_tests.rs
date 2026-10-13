// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Empty field lists retain their original construction syntax.
#![cfg(feature = "derive")]

use qubit_reflect::NamedConstructionInput;
use qubit_reflect::Reflect;
use qubit_reflect::StructUpdateInput;
use qubit_reflect::TupleConstructionInput;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::construct::ConstructionError;
use qubit_reflect::construct::ConstructionShape;
use qubit_reflect::descriptor::StructKind;
use qubit_reflect::descriptor::TypeDefinitionData;
use qubit_reflect::descriptor::TypeKind;

#[Reflect]
struct EmptyUnit;
#[Reflect]
struct EmptyNamed {
    // empty
}
#[Reflect]
struct EmptyTuple();

#[test]
fn test_empty_named_constructs_as_named() {
    let descriptor = TypeDescriptor::of::<EmptyNamed>();
    assert_eq!(descriptor.kind(), TypeKind::Struct(StructKind::Named));
    let output = descriptor
        .construct_struct::<EmptyNamed>(NamedConstructionInput::new())
        .expect("empty named struct construction should succeed");
    let _ = output;
}

#[Reflect]
struct GenericEmpty<const N: usize> {}
#[Reflect]
struct GenericTuple<const N: usize>();
#[Reflect]
struct GenericUnit<const N: usize>;

/// Checks construction and rejection across shapes without erasing type
/// identity.
fn assert_empty_shape<T: Reflect>(kind: StructKind) {
    let descriptor = TypeDescriptor::of::<T>();
    assert_eq!(descriptor.kind(), TypeKind::Struct(kind));
    let local = [
        descriptor.construct_unit::<T>(),
        descriptor.construct_struct::<T>(NamedConstructionInput::new()),
        descriptor.construct_tuple::<T>(TupleConstructionInput::new()),
    ];
    let shapes = [
        ConstructionShape::Unit,
        ConstructionShape::Named,
        ConstructionShape::Tuple,
    ];
    let expected = match kind {
        StructKind::Unit => ConstructionShape::Unit,
        StructKind::Named => ConstructionShape::Named,
        StructKind::Tuple => ConstructionShape::Tuple,
        StructKind::Newtype => panic!("empty structs cannot be newtypes"),
    };
    for (actual, result) in shapes.into_iter().zip(local) {
        if actual == expected {
            let updated = descriptor
                .update_struct::<T>(StructUpdateInput::new(
                    result.expect("matching local construction shape should succeed"),
                    NamedConstructionInput::new(),
                ))
                .expect("updating an empty struct with no fields should succeed");
            let _ = updated;
        } else {
            let Err(failure) = result else {
                panic!("wrong shape must fail")
            };
            assert!(
                matches!(failure.error(), ConstructionError::WrongShape { expected: e, actual: a }
                if *e == expected && *a == actual)
            );
        }
    }
}

#[test]
fn test_all_empty_shapes_construct_and_reject_wrong_shapes() {
    assert_empty_shape::<EmptyUnit>(StructKind::Unit);
    assert_empty_shape::<EmptyNamed>(StructKind::Named);
    assert_empty_shape::<EmptyTuple>(StructKind::Tuple);
}

/// Checks that the source definition and both concrete const instances agree.
fn assert_generic_shape<T: Reflect, U: Reflect>(kind: StructKind) {
    assert_empty_shape::<T>(kind);
    assert_empty_shape::<U>(kind);
    let first = TypeDescriptor::of::<T>();
    let second = TypeDescriptor::of::<U>();
    assert_ne!(first.type_id(), second.type_id());
    assert_eq!(
        first
            .generic_arguments()
            .expect("generic type should expose its const argument")
            .const_argument_value(0)
            .expect("first generic argument should be the const parameter")
            .downcast_ref::<usize>(),
        Some(&1)
    );
    assert_eq!(
        second
            .generic_arguments()
            .expect("generic type should expose its const argument")
            .const_argument_value(0)
            .expect("first generic argument should be the const parameter")
            .downcast_ref::<usize>(),
        Some(&2)
    );
    let definition = first
        .type_definition()
        .expect("generic struct should have a shared type definition");
    assert!(std::ptr::eq(
        definition,
        second
            .type_definition()
            .expect("second const instance should share the type definition")
    ));
    let TypeDefinitionData::Struct { kind: actual, fields } = definition.data() else {
        panic!("struct definition")
    };
    assert_eq!(*actual, kind);
    assert!(fields.is_empty());
}

#[test]
fn test_generic_empty_shapes_preserve_definition_and_const_arguments() {
    assert_generic_shape::<GenericUnit<1>, GenericUnit<2>>(StructKind::Unit);
    assert_generic_shape::<GenericEmpty<1>, GenericEmpty<2>>(StructKind::Named);
    assert_generic_shape::<GenericTuple<1>, GenericTuple<2>>(StructKind::Tuple);
}
