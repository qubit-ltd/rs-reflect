// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Integration tests for implementation and method declaration metadata.

use std::marker::PhantomData;

use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::descriptor::ImplDefinitionDescriptor;
use qubit_reflect::descriptor::ImplKind;
use qubit_reflect::ReflectionContext;

#[Reflect]
struct Service<T: 'static>(PhantomData<T>);

#[Reflect]
// This declaration exists to test trait implementation metadata.
#[allow(dead_code)]
trait ServiceContract<T> {
    fn convert<U: Clone>(&self, value: U) -> U;

    #[reflect(skip)]
    fn omitted_from_trait(&self);
}

#[ReflectImpl]
impl<T: 'static> ServiceContract<T> for Service<T> {
    fn convert<U: Clone>(&self, value: U) -> U {
        value
    }

    fn omitted_from_trait(&self) {}
}

#[ReflectImpl]
impl<T: 'static> Service<T> {
    // These methods are inspected through impl metadata rather than called.
    #[allow(dead_code)]
    pub fn inherent<U: Clone>(&self, value: U) -> U {
        value
    }

    #[reflect(skip)]
    #[allow(dead_code)]
    pub fn omitted_from_impl(&self) {}
}

fn implementation_definition_for(trait_name: Option<&str>) -> ImplDefinitionDescriptor<'static> {
    let context = ReflectionContext::global();
    context.initialize().expect("implementation fragments must validate");
    context
        .impl_definitions()
        .expect("implementation definitions are available")
        .iter()
        .copied()
        .find(|definition| match trait_name {
            Some(name) => {
                definition.kind() == ImplKind::Trait
                    && definition.implemented_trait().is_some_and(|trait_| {
                        trait_.path().ends_with(name)
                    })
            }
            None => {
                definition.kind() == ImplKind::Inherent
                    && definition
                        .methods()
                        .iter()
                        .any(|method| method.rust_name() == "inherent")
            }
        })
        .expect("the requested implementation definition must be registered")
}

#[test]
fn test_reflect_impl_preserves_method_declarations_and_generic_parameters() {
    let implementation = implementation_definition_for(None);
    let methods = implementation.methods();

    assert_eq!(methods.len(), 1);
    assert_eq!(methods[0].rust_name(), "inherent");
    assert_eq!(methods[0].generic_definition().parameters().len(), 1);
}

#[test]
fn test_reflect_impl_skip_omits_method_without_changing_trait_declaration() {
    let context = ReflectionContext::global();
    context
        .initialize()
        .expect("trait declaration fragments must validate");
    let trait_definition = context
        .trait_definition_by_name(concat!(module_path!(), "::ServiceContract"))
        .expect("the trait declaration remains registered");
    let trait_methods: Vec<_> = trait_definition
        .methods()
        .iter()
        .map(|method| method.rust_name())
        .collect();
    assert_eq!(trait_methods, ["convert"]);

    let implementation = implementation_definition_for(Some("ServiceContract"));
    let method_names: Vec<_> = implementation
        .methods()
        .iter()
        .map(|method| method.rust_name())
        .collect();
    assert_eq!(method_names, ["convert"]);
}
