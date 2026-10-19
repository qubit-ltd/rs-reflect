// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0 (the "License");
//    you may not use this file except in compliance with the License.
// =============================================================================

//! Integration tests for vNext generic definitions and concrete arguments.

use std::rc::Rc;

use qubit_reflect::Reflect;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::descriptor::{GenericArgumentDescriptor, GenericParameterKind};

#[Reflect]
struct User {
    id: u64,
}

#[Reflect]
struct Envelope<'a, T, const N: usize> {
    label: &'a str,
    item: T,
    bytes: [u8; N],
}

#[Reflect]
struct GenericHolder<T> {
    item: T,
}

/// Verifies concrete type and const arguments map to their declared generic
/// parameters.
#[test]
fn test_generic_descriptor_exposes_concrete_type_and_const_arguments() {
    let descriptor = TypeDescriptor::of::<Envelope<'static, User, 4>>();
    let arguments = descriptor.generic_arguments();
    let value = Envelope {
        label: "sample",
        item: User { id: 1 },
        bytes: [0; 4],
    };

    assert_eq!(value.label, "sample");
    assert_eq!(value.item.id, 1);
    assert_eq!(value.bytes.len(), 4);
    assert_eq!(arguments.iter().len(), 2);
    let GenericArgumentDescriptor::Type(item) = arguments.get("T").expect("type argument exists") else {
        panic!("T is a type argument");
    };
    assert_eq!(item.find_concrete_type().unwrap().type_id(), std::any::TypeId::of::<User>());

    let GenericArgumentDescriptor::Const(width) = arguments.get("N").expect("const argument exists") else {
        panic!("N is a const argument");
    };
    assert!(matches!(width.expression(), qubit_reflect::expression::ConstExpression::UnsignedInteger(4)));
}

/// Verifies lifetime parameters remain part of the source definition and are
/// excluded from concrete type identity arguments.
#[test]
fn test_generic_definition_retains_lifetime_parameters() {
    let descriptor = TypeDescriptor::of::<Envelope<'static, User, 4>>();
    let definition = descriptor.definition().expect("Envelope has a type definition");
    let parameters = definition.generics();

    assert!(matches!(parameters.parameter("a").unwrap().kind(), GenericParameterKind::Lifetime { .. }));
    assert_eq!(descriptor.generic_arguments().iter().len(), 2);
}

/// Verifies an unreflected generic type argument is retained as metadata
/// without imposing an unnecessary `Reflect` bound.
#[test]
fn test_unreflectable_generic_argument_remains_non_navigable() {
    let descriptor = TypeDescriptor::of::<GenericHolder<Rc<()>>>();
    let GenericArgumentDescriptor::Type(item) = descriptor
        .generic_arguments()
        .get("T")
        .expect("generic type argument is retained")
    else {
        panic!("T is a type argument");
    };

    assert!(item.find_concrete_type().is_none());
    assert!(descriptor.field("item").is_ok());
    let value = GenericHolder { item: Rc::new(()) };
    let _ = value.item;
}
