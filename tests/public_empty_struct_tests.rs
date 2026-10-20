// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Empty field lists retain their source shape and construct through vNext.
#![cfg(feature = "derive")]

use qubit_reflect::Reflect;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::construct::ConstructionError;
use qubit_reflect::construct::ConstructionShape;
use qubit_reflect::construct::NamedFields;
use qubit_reflect::construct::PositionalFields;
use qubit_reflect::construct::UnitFields;
use qubit_reflect::construct::UpdateInput;
use qubit_reflect::descriptor::ConstExpression;
use qubit_reflect::descriptor::StructKind;
use qubit_reflect::descriptor::TypeDefinitionKind;
use qubit_reflect::value::Local;

#[Reflect]
struct EmptyUnit;
#[Reflect]
struct EmptyNamed {
    // empty
}
#[Reflect]
struct EmptyTuple();

#[Reflect]
struct GenericEmpty<const N: usize> {}
#[Reflect]
struct GenericTuple<const N: usize>();
#[Reflect]
struct GenericUnit<const N: usize>;

#[test]
fn test_empty_named_constructs_as_named() {
    let descriptor = TypeDescriptor::of::<EmptyNamed>();
    assert!(matches!(
        descriptor.definition().expect("source definition exists").kind(),
        TypeDefinitionKind::Struct(StructKind::Named)
    ));
    assert_eq!(descriptor.construction().shape(), ConstructionShape::Named);
    assert!(descriptor.construction().fields().is_empty());
    let output: EmptyNamed = descriptor
        .construct(NamedFields::<Local>::new())
        .expect("empty named struct construction should succeed");
    let _ = output;
}

/// Checks construction and rejection across shapes without erasing type
/// identity.
fn assert_empty_shape<T: qubit_reflect::__private::codegen_v4::descriptor::Reflect>(
    kind: StructKind,
    expected: ConstructionShape,
) {
    let descriptor = TypeDescriptor::of::<T>();
    assert_eq!(
        descriptor.definition().expect("source definition exists").kind(),
        TypeDefinitionKind::Struct(kind)
    );
    let construction = descriptor.construction();
    assert_eq!(construction.shape(), expected);
    assert!(construction.fields().is_empty());

    let local = [
        descriptor.construct::<T, _>(UnitFields::new()).map(|_| ()),
        descriptor.construct::<T, _>(NamedFields::<Local>::new()).map(|_| ()),
        descriptor
            .construct::<T, _>(PositionalFields::<Local>::new())
            .map(|_| ()),
    ];
    let shapes = [
        ConstructionShape::Unit,
        ConstructionShape::Named,
        ConstructionShape::Positional,
    ];
    for (actual, result) in shapes.into_iter().zip(local) {
        if actual == expected {
            result.expect("matching empty input shape should construct");
        } else {
            let failure = result.expect_err("mismatched input shape must fail");
            assert!(matches!(
                failure.error(),
                ConstructionError::ShapeMismatch { expected: e, actual: a }
                    if *e == expected && *a == actual
            ));
        }
    }

    let base = match expected {
        ConstructionShape::Unit => descriptor.construct::<T, _>(UnitFields::new()),
        ConstructionShape::Named => descriptor.construct::<T, _>(NamedFields::<Local>::new()),
        ConstructionShape::Positional => descriptor.construct::<T, _>(PositionalFields::<Local>::new()),
    }
    .expect("matching input creates the value used for the empty update");
    descriptor
        .update::<T, Local>(UpdateInput::<Local>::new(base))
        .expect("updating an empty struct with no changed fields should succeed");
}

#[test]
fn test_all_empty_shapes_construct_and_reject_wrong_shapes() {
    assert_empty_shape::<EmptyUnit>(StructKind::Unit, ConstructionShape::Unit);
    assert_empty_shape::<EmptyNamed>(StructKind::Named, ConstructionShape::Named);
    assert_empty_shape::<EmptyTuple>(StructKind::Tuple, ConstructionShape::Positional);
}

/// Checks source definitions and concrete const instances through vNext
/// descriptors.
fn assert_generic_shape<
    T: qubit_reflect::__private::codegen_v4::descriptor::Reflect,
    U: qubit_reflect::__private::codegen_v4::descriptor::Reflect,
>(
    kind: StructKind,
) {
    let first = TypeDescriptor::of::<T>();
    let second = TypeDescriptor::of::<U>();
    assert_eq!(
        first.definition().expect("generic source definition exists").kind(),
        TypeDefinitionKind::Struct(kind)
    );
    assert_ne!(first.type_id(), second.type_id());
    assert_eq!(
        first
            .generic_arguments()
            .argument("N")
            .expect("const argument is retained")
            .const_argument()
            .expect("N is a const parameter")
            .expression(),
        ConstExpression::UnsignedInteger(1)
    );
    assert_eq!(
        second
            .generic_arguments()
            .argument("N")
            .expect("const argument is retained")
            .const_argument()
            .expect("N is a const parameter")
            .expression(),
        ConstExpression::UnsignedInteger(2)
    );
    assert_eq!(
        first.definition().expect("definition exists").id(),
        second.definition().expect("definition exists").id()
    );
}

#[test]
fn test_generic_empty_shapes_preserve_definition_and_const_arguments() {
    assert_generic_shape::<GenericUnit<1>, GenericUnit<2>>(StructKind::Unit);
    assert_generic_shape::<GenericEmpty<1>, GenericEmpty<2>>(StructKind::Named);
    assert_generic_shape::<GenericTuple<1>, GenericTuple<2>>(StructKind::Tuple);
}
