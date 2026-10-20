// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// =============================================================================

//! Integration tests for vNext generic definitions and concrete arguments.

use std::rc::Rc;

use qubit_reflect::Reflect;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::descriptor::GenericParameterKind;

#[Reflect]
struct User {
    id: u64,
}

#[Reflect]
struct Envelope<'scope, T, const N: usize> {
    marker: std::marker::PhantomData<&'scope ()>,
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
        marker: std::marker::PhantomData,
        item: User { id: 1 },
        bytes: [0; 4],
    };

    assert_eq!(std::mem::size_of_val(&value.marker), 0);
    assert_eq!(value.item.id, 1);
    assert_eq!(value.bytes.len(), 4);
    assert_eq!(arguments.iter().len(), 2);
    let item = arguments
        .get("T")
        .expect("type argument exists")
        .type_argument()
        .expect("T is a type argument");
    assert_eq!(
        item.find_concrete_type().unwrap().type_id(),
        std::any::TypeId::of::<User>()
    );

    let width = arguments
        .get("N")
        .expect("const argument exists")
        .const_argument()
        .expect("N is a const argument");
    assert_eq!(format!("{:?}", width.expression()), "UnsignedInteger(4)");
}

/// Verifies lifetime parameters remain part of the source definition and are
/// excluded from concrete type identity arguments.
#[test]
fn test_generic_definition_retains_lifetime_parameters() {
    let descriptor = TypeDescriptor::of::<Envelope<'static, User, 4>>();
    let definition = descriptor.definition().expect("Envelope has a type definition");
    let parameters = definition.generics();

    let lifetime = parameters.parameters().next().expect("lifetime parameter exists");
    assert_eq!(lifetime.name(), "scope");
    assert!(matches!(lifetime.kind(), GenericParameterKind::Lifetime { .. }));
    assert_eq!(descriptor.generic_arguments().iter().len(), 2);
}

/// Verifies a concrete reflected generic type argument remains navigable from
/// the context-bound descriptor.
#[test]
fn test_reflected_generic_argument_has_a_concrete_descriptor() {
    let descriptor = TypeDescriptor::of::<GenericHolder<Rc<()>>>();
    let item = descriptor
        .generic_arguments()
        .get("T")
        .expect("generic type argument is retained")
        .type_argument()
        .expect("T is a type argument");

    assert_eq!(
        item.find_concrete_type().expect("Rc is reflectable").type_id(),
        std::any::TypeId::of::<Rc<()>>()
    );
    assert!(descriptor.field("item").is_ok());
    let value = GenericHolder { item: Rc::new(()) };
    let _ = value.item;
}
