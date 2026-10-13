// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Coverage for explicitly bound default trait method invocation.

use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::bind_method;
use qubit_reflect::descriptor::MethodLookup;
use qubit_reflect::identity::FragmentIdentity;
use qubit_reflect::invoke::Invocation;
use qubit_reflect::invoke::InvocationOutput;
use qubit_reflect::registry::RegistrySnapshotBuilder;
use qubit_reflect::value::DynamicOwned;
use qubit_reflect::value::Local;

#[Reflect]
struct DefaultService;

#[Reflect]
pub trait DefaultRenderer {
    fn render(&self, input: String) -> String {
        format!("rendered: {input}")
    }
}

#[ReflectImpl(definition_provider_v2 = __qubit_reflect_trait_definition_DefaultRenderer)]
impl DefaultRenderer for DefaultService {}

#[test]
fn test_default_trait_method_can_be_called_after_explicit_binding() {
    let binding = bind_method!(<DefaultService as DefaultRenderer>::render as fn(&DefaultService, String) -> String);
    let mut builder = RegistrySnapshotBuilder::new();
    builder.include_linked_fragments();
    builder.add_method_binding(
        binding,
        FragmentIdentity::new("default-trait-method-test", module_path!(), line!(), 1, "binding", 1),
    );
    let registry = builder
        .build()
        .expect("the binding matches the default method declaration");
    let service = DefaultService;

    let MethodLookup::Unique(method) = DefaultService::type_descriptor().methods_named_in(&registry, "render") else {
        panic!("the default trait method must resolve to one bound method instance");
    };
    let output = method
        .invoke(
            &registry,
            Invocation::receiver_ref(&service).arg(String::from("example")),
        )
        .expect("the bound default method accepts a typed input");
    let InvocationOutput::Owned(value) = output else {
        panic!("the default method returns an owned string");
    };
    assert_eq!(
        DynamicOwned::<Local>::downcast_ref::<String>(&value).map(String::as_str),
        Some("rendered: example")
    );
}
