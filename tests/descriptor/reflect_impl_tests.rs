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
use qubit_reflect::ReflectionContext;
use qubit_reflect::descriptor::ImplDefinitionDescriptor;
use qubit_reflect::descriptor::ImplDefinitionId;
use qubit_reflect::descriptor::ImplKind;
use qubit_reflect::descriptor::TraitDefinitionId;

#[Reflect]
struct Service<T: 'static>(#[reflect(skip)] PhantomData<T>);

#[Reflect]
// This declaration exists to test trait implementation metadata.
#[allow(dead_code)]
trait ServiceContract<T> {
    fn convert<U: Clone>(&self, value: U) -> U;

    fn omitted_from_trait(&self);
}

#[ReflectImpl(reflected_trait = ServiceContract::<T>)]
impl<T: 'static> ServiceContract<T> for Service<T> {
    fn convert<U: Clone>(&self, value: U) -> U {
        value
    }

    #[reflect(skip)]
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
    ReflectionContext::global()
        .impl_definitions()
        .expect("source implementation definitions are available")
        .iter()
        .find(|definition| match trait_name {
            Some(name) => {
                definition.kind() == ImplKind::Trait
                    && definition
                        .implemented_trait()
                        .is_some_and(|trait_| trait_.path().contains(name))
                    && definition.methods().iter().any(|method| method.name() == "convert")
            }
            None => {
                definition.kind() == ImplKind::Inherent
                    && definition.methods().iter().any(|method| method.name() == "inherent")
            }
        })
        .expect("the requested implementation definition must be registered")
}

#[test]
fn test_reflect_impl_preserves_method_declarations_and_generic_parameters() {
    let implementation = implementation_definition_for(None);
    let methods = implementation.methods();

    assert_eq!(methods.len(), 1);
    let method = methods
        .get("inherent")
        .expect("the inherent method declaration is present");
    assert_eq!(method.name(), "inherent");
    assert_eq!(method.generics().parameters().len(), 1);
}

#[test]
fn test_reflect_impl_skip_omits_implementation_method_without_changing_trait_declaration() {
    let context = ReflectionContext::global();
    context.initialize().expect("trait declaration fragments must validate");
    let trait_definition = context
        .trait_definition_by_name(concat!(module_path!(), "::ServiceContract"))
        .expect("the trait declaration remains registered");
    let trait_methods: Vec<_> = trait_definition.methods().iter().map(|method| method.name()).collect();
    assert_eq!(trait_methods, ["convert", "omitted_from_trait"]);

    let implementation = implementation_definition_for(Some("ServiceContract"));
    let method_names: Vec<_> = implementation.methods().iter().map(|method| method.name()).collect();
    assert_eq!(method_names, ["convert"]);
}

#[test]
fn test_context_lists_source_impl_definitions() {
    let context = ReflectionContext::global();
    context.initialize().expect("implementation metadata must validate");
    let definitions = context
        .impl_definitions()
        .expect("registered source implementation declarations are queryable");

    assert!(definitions.iter().any(|definition| {
        definition.kind() == ImplKind::Inherent && definition.methods().iter().any(|method| method.name() == "inherent")
    }));
    assert!(definitions.iter().any(|definition| {
        definition.kind() == ImplKind::Trait && definition.methods().iter().any(|method| method.name() == "convert")
    }));
}

#[test]
fn test_definition_ids_are_public_and_comparable() {
    let context = ReflectionContext::global();
    context.initialize().expect("definition metadata must initialize");

    let trait_definition = context
        .trait_definition_by_name(concat!(module_path!(), "::ServiceContract"))
        .expect("the reflected trait definition is registered");
    let trait_id: TraitDefinitionId = trait_definition.id();
    assert_eq!(trait_id, trait_definition.id());

    let impl_definition = context
        .impl_definitions()
        .expect("implementation definitions are available")
        .iter()
        .find(|definition| {
            definition.kind() == ImplKind::Trait
                && definition
                    .implemented_trait()
                    .is_some_and(|trait_| trait_.path().ends_with("ServiceContract"))
        })
        .expect("the trait implementation definition is registered");
    let impl_id: ImplDefinitionId = impl_definition.id();
    assert_eq!(impl_id, impl_definition.id());
}
