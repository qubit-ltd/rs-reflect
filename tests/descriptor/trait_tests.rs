// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0 (the "License");
//    you may not use this file except in compliance with the License.
// =============================================================================

//! Integration tests for context-bound trait and impl descriptors.

use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::ReflectionContext;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::descriptor::{GenericArgumentDescriptor, MethodImplementationSource};
use qubit_reflect::lookup::Lookup;

#[Reflect]
struct Service;

#[Reflect]
trait ServiceContract {
    type Output: Reflect;

    const LIMIT: u32;

    fn execute(&self, input: u32) -> Self::Output;
}

#[ReflectImpl]
impl ServiceContract for Service {
    type Output = u64;

    const LIMIT: u32 = 16;

    fn execute(&self, input: u32) -> Self::Output {
        u64::from(input)
    }
}

#[Reflect]
trait GenericSource<T> {
    fn source(&self) -> T;
}

#[ReflectImpl]
impl GenericSource<u8> for Service {
    fn source(&self) -> u8 {
        1
    }
}

#[ReflectImpl]
impl GenericSource<String> for Service {
    fn source(&self) -> String {
        String::from("value")
    }
}

#[Reflect]
trait SameNameTrait {
    fn duplicate(&self) -> u8;
}

#[Reflect]
trait DefaultMethodContract {
    fn required(&self);

    fn status(&self) -> u8 {
        1
    }
}

#[Reflect]
struct DefaultMethodService;

#[ReflectImpl]
impl DefaultMethodContract for DefaultMethodService {
    fn required(&self) {}
}

#[Reflect]
struct OverriddenMethodService;

#[ReflectImpl]
impl DefaultMethodContract for OverriddenMethodService {
    fn required(&self) {}

    fn status(&self) -> u8 {
        2
    }
}

#[ReflectImpl]
impl SameNameTrait for Service {
    fn duplicate(&self) -> u8 {
        2
    }
}

#[ReflectImpl]
impl Service {
    fn duplicate(&self) -> u8 {
        3
    }
}

/// Verifies a trait definition exposes source methods and associated items.
#[test]
fn test_trait_definition_preserves_declarations() {
    let context = ReflectionContext::global();
    context.initialize().expect("linked trait metadata validates");
    let definition = context
        .trait_definition_by_name(concat!(module_path!(), "::ServiceContract"))
        .expect("the reflected trait definition is discoverable");

    assert_eq!(definition.name(), "ServiceContract");
    assert_eq!(definition.path(), concat!(module_path!(), "::ServiceContract"));
    assert_eq!(definition.methods().len(), 1);
    assert_eq!(definition.method("execute").unwrap().name(), "execute");
    assert_eq!(definition.associated_types().len(), 1);
    assert_eq!(definition.associated_type("Output").unwrap().name(), "Output");
    assert_eq!(definition.associated_consts().len(), 1);
    assert_eq!(definition.associated_const("LIMIT").unwrap().name(), "LIMIT");
}

/// Verifies a concrete type exposes its reflected trait implementation and
/// associated bindings.
#[test]
fn test_trait_impl_preserves_application_and_associated_bindings() {
    let descriptor = TypeDescriptor::of::<Service>();
    let implementation = descriptor
        .trait_impls()
        .expect("linked trait implementations are available")
        .iter()
        .find(|implementation| {
            implementation
                .implemented_trait()
                .is_some_and(|trait_| trait_.name() == "ServiceContract")
        })
        .expect("the service contract implementation is present");

    assert_eq!(implementation.target().type_id(), descriptor.type_id());
    assert_eq!(implementation.method("execute").unwrap().name(), "execute");
    assert_eq!(implementation.associated_types().len(), 1);
    assert_eq!(implementation.associated_type("Output").unwrap().definition().name(), "Output");
    assert_eq!(implementation.associated_consts().len(), 1);
    assert_eq!(implementation.associated_const("LIMIT").unwrap().definition().name(), "LIMIT");
}

/// Verifies generic trait applications remain distinct on one target type.
#[test]
fn test_generic_trait_implementations_preserve_each_application() {
    let implementations: Vec<_> = TypeDescriptor::of::<Service>()
        .trait_impls()
        .expect("linked trait implementations are available")
        .iter()
        .filter(|implementation| {
            implementation
                .implemented_trait()
                .is_some_and(|trait_| trait_.name() == "GenericSource")
        })
        .collect();

    assert_eq!(implementations.len(), 2);
    let mut type_arguments = Vec::new();
    for implementation in implementations {
        let application = implementation.implemented_trait().expect("trait application exists");
        let GenericArgumentDescriptor::Type(argument) = application
            .arguments()
            .get("T")
            .expect("the generic trait application retains T")
        else {
            panic!("T is a type argument");
        };
        type_arguments.push(argument.find_concrete_type().unwrap().type_id());
        assert_eq!(implementation.method("source").unwrap().name(), "source");
    }
    assert_eq!(type_arguments.len(), 2);
    assert!(type_arguments.contains(&std::any::TypeId::of::<String>()));
    assert!(type_arguments.contains(&std::any::TypeId::of::<u8>()));
}

/// Verifies an impl exposes whether a trait default body was inherited or
/// replaced by a concrete method.
#[test]
fn test_trait_method_reports_default_and_override_sources() {
    let defaulted = TypeDescriptor::of::<DefaultMethodService>()
        .trait_impls()
        .expect("linked implementations are available")
        .iter()
        .find(|implementation| implementation.implemented_trait().is_some())
        .expect("the default method trait implementation is linked")
        .method("status")
        .expect("the inherited default method is effective");
    let overridden = TypeDescriptor::of::<OverriddenMethodService>()
        .trait_impls()
        .expect("linked implementations are available")
        .iter()
        .find(|implementation| implementation.implemented_trait().is_some())
        .expect("the overriding trait implementation is linked")
        .method("status")
        .expect("the overridden method is effective");

    assert_eq!(defaulted.implementation_source(), MethodImplementationSource::Defaulted);
    assert_eq!(overridden.implementation_source(), MethodImplementationSource::Overridden);
}

/// Verifies methods from separate namespaces retain explicit ambiguity.
#[test]
fn test_trait_and_inherent_methods_keep_separate_namespaces() {
    let descriptor = TypeDescriptor::of::<Service>();

    assert!(matches!(descriptor.lookup_method("duplicate").unwrap(), Lookup::Ambiguous(_)));
    assert!(matches!(descriptor.lookup_method("missing").unwrap(), Lookup::Missing));
}
