// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![cfg(feature = "derive")]

use std::any::TypeId;

use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::descriptor::AssociatedTypeBindingDescriptor;
use qubit_reflect::descriptor::ImplDescriptor;
use qubit_reflect::expression::AssociatedTypeExpression;
use qubit_reflect::expression::ConcreteTypeExpression;
use qubit_reflect::expression::TypeExpression;
use qubit_reflect::registry::ReflectRegistry;

#[Reflect]
trait AssociatedContract {
    type Output: Reflect;

    const VALUE: u32;
}

#[Reflect]
struct AssociatedService;

#[ReflectImpl(definition_provider_v2 = __qubit_reflect_trait_definition_AssociatedContract)]
impl AssociatedContract for AssociatedService {
    type Output = u16;

    const VALUE: u32 = 23;
}

fn implementation() -> &'static ImplDescriptor {
    ReflectRegistry::initialize()
        .expect("associated item fragments must validate")
        .implementations(TypeDescriptor::of::<AssociatedService>().type_id())
        .iter()
        .copied()
        .find(|implementation| {
            implementation
                .implemented_trait()
                .is_some_and(|trait_descriptor| trait_descriptor.rust_name() == "AssociatedContract")
        })
        .expect("the associated trait implementation is registered")
}

#[test]
fn associated_const_reader_checks_the_exact_requested_type() {
    let implementation = implementation();
    let [value] = implementation.associated_consts() else {
        panic!("the associated constant binding is present")
    };

    assert_eq!(value.read::<u32>(), Some(Ok(23)));
    let mismatch = value
        .read::<u8>()
        .expect("the associated constant has a reader")
        .expect_err("a reader must reject a different requested type");
    assert_eq!(mismatch.expected(), TypeId::of::<u8>());
    assert_eq!(mismatch.actual(), TypeId::of::<u32>());
}

#[test]
fn symbolic_associated_type_binding_retains_its_unresolved_projection() {
    let registry = ReflectRegistry::initialize().expect("trait definition fragments must validate");
    let definition = registry
        .trait_definition_by_path(concat!(module_path!(), "::AssociatedContract"))
        .expect("the associated type declaration is registered");
    let [declaration] = definition.associated_types() else {
        panic!("the trait declares one associated type")
    };
    assert_eq!(declaration.rust_name(), "Output");
    assert!(declaration.default().is_none());
    let trait_path = TypeExpression::Concrete(
        ConcreteTypeExpression::new(["example", "Projectable"], []).expect("a trait projection path is non-empty"),
    );
    let projection = TypeExpression::Associated(AssociatedTypeExpression::new(
        TypeExpression::parameter("T").expect("the projection parameter is named"),
        Some(trait_path),
        "Output",
        [],
    ));
    let binding = AssociatedTypeBindingDescriptor::new(declaration, projection.clone(), None);

    assert_eq!(binding.value(), &projection);
    assert!(matches!(binding.value(), TypeExpression::Associated(_)));
    assert!(binding.concrete_type().is_none());
}
