// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Empty field lists retain their original construction syntax.
#![cfg(feature = "derive")]

use qubit_reflect::construct::ConstructionError;
use qubit_reflect::construct::ConstructionShape;
use qubit_reflect::descriptor::StructKind;
use qubit_reflect::descriptor::TypeDefinitionData;
use qubit_reflect::descriptor::TypeKind;
use qubit_reflect::value::DynamicOwned;
use qubit_reflect::value::ThreadSafe;
use qubit_reflect::NamedConstructionInput;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectedOwned;
use qubit_reflect::StructUpdateInput;
use qubit_reflect::TupleConstructionInput;
use qubit_reflect::TypeDescriptor;

#[derive(Reflect)]
#[reflect(thread_safe)]
struct EmptyUnit;
#[derive(Reflect)]
#[reflect(thread_safe)]
struct EmptyNamed {
    // empty
}
#[derive(Reflect)]
#[reflect(thread_safe)]
struct EmptyTuple();

#[test]
fn test_empty_named_constructs_as_named() {
    let descriptor = TypeDescriptor::of::<EmptyNamed>();
    assert_eq!(descriptor.kind(), TypeKind::Struct(StructKind::Named));
    let output = descriptor
        .construct_struct(NamedConstructionInput::new(std::iter::empty::<(&str, ReflectedOwned)>()))
        .expect("empty named struct construction should succeed");
    assert!(output.downcast::<EmptyNamed>().is_ok());
}

#[derive(Reflect)]
#[reflect(thread_safe)]
struct GenericEmpty<const N: usize> {}
#[derive(Reflect)]
#[reflect(thread_safe)]
struct GenericTuple<const N: usize>();
#[derive(Reflect)]
#[reflect(thread_safe)]
struct GenericUnit<const N: usize>;

/// Checks construction and rejection in both modes without erasing type
/// identity.
fn assert_empty_shape<T: Reflect>(kind: StructKind) {
    let descriptor = TypeDescriptor::of::<T>();
    assert_eq!(descriptor.kind(), TypeKind::Struct(kind));
    let local = [
        descriptor.construct_unit(),
        descriptor.construct_struct(NamedConstructionInput::new(std::iter::empty::<(&str, ReflectedOwned)>())),
        descriptor.construct_tuple(TupleConstructionInput::new(std::iter::empty::<ReflectedOwned>())),
    ];
    let constructor = descriptor
        .struct_construction()
        .expect("empty struct descriptor should expose construction")
        .thread_safe_constructor()
        .expect("empty struct should have a thread-safe constructor");
    let threaded = [
        constructor.construct_unit(),
        constructor.construct_named(NamedConstructionInput::new(std::iter::empty::<(
            &str,
            DynamicOwned<ThreadSafe>,
        )>())),
        constructor.construct_tuple(TupleConstructionInput::new(
            std::iter::empty::<DynamicOwned<ThreadSafe>>(),
        )),
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
                .struct_construction()
                .expect("empty struct descriptor should expose construction")
                .local_updater()
                .expect("empty struct should have a local updater")
                .update(StructUpdateInput::new(
                    result.expect("matching local construction shape should succeed"),
                    NamedConstructionInput::new(std::iter::empty::<(&str, ReflectedOwned)>()),
                ))
                .expect("updating an empty struct with no fields should succeed");
            assert!(updated.downcast::<T>().is_ok());
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
    for (actual, result) in shapes.into_iter().zip(threaded) {
        if actual == expected {
            assert!(
                result
                    .expect("matching thread-safe construction shape should succeed")
                    .downcast::<T>()
                    .is_ok()
            );
        } else {
            let Err(failure) = result else {
                panic!("wrong thread-safe shape must fail")
            };
            assert!(
                matches!(failure.error(), ConstructionError::WrongShape { expected: e, actual: a }
                if *e == expected && *a == actual)
            );
        }
    }
}

#[test]
fn test_all_empty_shapes_construct_in_both_modes_and_reject_wrong_shapes() {
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
