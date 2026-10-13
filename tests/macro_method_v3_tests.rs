// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![cfg(feature = "derive")]

use std::pin::Pin;

use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::bind_method;
use qubit_reflect::descriptor::MethodLookup;
use qubit_reflect::descriptor::MethodQualifier;
use qubit_reflect::identity::FragmentIdentity;
use qubit_reflect::invoke::Invocation;
use qubit_reflect::invoke::InvocationOutput;
use qubit_reflect::registry::RegistrySnapshotBuilder;
use qubit_reflect::value::DynamicOwned;
use qubit_reflect::value::Local;

#[Reflect]
pub struct Service<T>(pub T);

#[Reflect]
pub struct Utility;

pub struct PinUtility(pub u8);

impl qubit_reflect::Reflect for PinUtility {
    fn type_descriptor() -> &'static qubit_reflect::TypeDescriptor {
        static DESCRIPTOR: std::sync::OnceLock<qubit_reflect::TypeDescriptor> = std::sync::OnceLock::new();
        DESCRIPTOR.get_or_init(|| {
            qubit_reflect::__private::codegen_v3::descriptor::struct_type::<PinUtility>(
                "PinUtility",
                qubit_reflect::descriptor::StructKind::Tuple,
                &[],
            )
        })
    }
}

#[ReflectImpl]
impl<T> Service<T> {
    pub fn pair<U: ToString>(&self, value: U) -> String {
        value.to_string()
    }

    pub fn const_method<const N: usize>(&self, value: String) -> usize {
        value.len() + N
    }

    pub fn replace(&mut self, value: T) {
        self.0 = value;
    }
}

#[ReflectImpl]
impl Utility {
    pub fn associated(value: String) -> String {
        value
    }
    pub fn borrowed(value: &String) -> &String {
        value
    }
    pub async fn async_owned(value: String) -> String {
        value
    }
}

#[ReflectImpl]
impl PinUtility {
    pub fn set_pin(self: Pin<&mut Self>, value: u8) {
        self.get_mut().0 = value;
    }
}

#[Reflect]
pub trait Render<T> {
    fn render<U: ToString>(&self, value: U) -> String;
}

#[Reflect]
pub trait Paint<T> {
    fn render<U: ToString>(&self, value: U) -> String;
}

#[ReflectImpl(definition_provider_v2 = __qubit_reflect_trait_definition_Render)]
impl<T> Render<T> for Service<T> {
    fn render<U: ToString>(&self, value: U) -> String {
        value.to_string()
    }
}

#[ReflectImpl(definition_provider_v2 = __qubit_reflect_trait_definition_Paint)]
impl<T> Paint<T> for Service<T> {
    fn render<U: ToString>(&self, value: U) -> String {
        format!("paint:{}", value.to_string())
    }
}

#[test]
fn explicit_signature_binds_generic_impl_and_method_arguments() {
    let _binding = bind_method!(Service::<u8>::pair::<String> as fn(&Service<u8>, String) -> String);
}

#[test]
fn explicit_signature_binds_const_generic_method_arguments() {
    let _binding = bind_method!(Service::<u8>::const_method::<3> as fn(&Service<u8>, String) -> usize);
}

#[test]
fn ufcs_binding_retains_applied_trait_identity() {
    let _binding = bind_method!(<Service<u8> as Render<u8>>::render::<String> as fn(&Service<u8>, String) -> String);
}

#[test]
fn bound_method_adapter_resolves_and_invokes_in_an_explicit_snapshot() {
    let mut builder = RegistrySnapshotBuilder::new();
    builder.include_linked_fragments();
    builder.add_method_binding(
        bind_method!(Service::<u8>::pair::<String> as fn(&Service<u8>, String) -> String),
        FragmentIdentity::new("macro-method-test", module_path!(), line!(), 1, "binding", 1),
    );
    builder.add_method_binding(
        bind_method!(Service::<u8>::const_method::<3> as fn(&Service<u8>, String) -> usize),
        FragmentIdentity::new("macro-method-test", module_path!(), line!(), 1, "binding", 2),
    );
    builder.add_method_binding(
        bind_method!(<Service<u8> as Render<u8>>::render::<String> as fn(&Service<u8>, String) -> String),
        FragmentIdentity::new("macro-method-test", module_path!(), line!(), 1, "binding", 3),
    );
    builder.add_method_binding(
        bind_method!(Service::<u8>::replace as fn(&mut Service<u8>, u8)),
        FragmentIdentity::new("macro-method-test", module_path!(), line!(), 1, "binding", 4),
    );
    builder.add_method_binding(
        bind_method!(Utility::associated as fn(String) -> String),
        FragmentIdentity::new("macro-method-test", module_path!(), line!(), 1, "binding", 5),
    );
    builder.add_method_binding(
        bind_method!(Utility::borrowed as fn(&String) -> &String),
        FragmentIdentity::new("macro-method-test", module_path!(), line!(), 1, "binding", 6),
    );
    builder.add_method_binding(
        bind_method!(Utility::async_owned as async fn(String) -> String),
        FragmentIdentity::new("macro-method-test", module_path!(), line!(), 1, "binding", 7),
    );
    builder.add_method_binding(
        bind_method!(PinUtility::set_pin as fn(Pin<&mut PinUtility>, u8)),
        FragmentIdentity::new("macro-method-test", module_path!(), line!(), 1, "binding", 8),
    );
    let registry = builder.build().expect("linked declarations and bindings match");
    for binding in registry.method_bindings() {
        let resolved = registry.method_binding(
            binding.target().type_id(),
            binding.impl_identity(),
            binding.declaration_identity(),
            binding.method_arguments(),
        );
        assert!(resolved.is_some_and(|resolved| std::ptr::eq(resolved, binding)));
    }
    let service = Service(7_u8);

    let MethodLookup::Unique(pair) = Service::<u8>::type_descriptor().methods_named_in(&registry, "pair") else {
        panic!("bound generic method should be visible");
    };
    let output = pair
        .invoke(&registry, Invocation::receiver_ref(&service).arg(String::from("pair")))
        .expect("bound adapter accepts arguments matching its explicit signature");
    let InvocationOutput::Owned(value) = output else {
        panic!("method returns an owned string")
    };
    assert_eq!(
        DynamicOwned::<Local>::downcast_ref(&value).map(String::as_str),
        Some("pair")
    );

    let MethodLookup::Unique(const_method) =
        Service::<u8>::type_descriptor().methods_named_in(&registry, "const_method")
    else {
        panic!("bound const-generic method should be visible");
    };
    let output = const_method
        .invoke(&registry, Invocation::receiver_ref(&service).arg(String::from("ab")))
        .expect("bound adapter accepts arguments matching its explicit signature");
    let InvocationOutput::Owned(value) = output else {
        panic!("method returns an owned integer")
    };
    assert_eq!(DynamicOwned::<Local>::downcast_ref(&value), Some(&5_usize));

    let MethodLookup::Unique(render) = Service::<u8>::type_descriptor().methods_named_in(&registry, "render") else {
        panic!("bound trait method should be visible");
    };
    let output = render
        .invoke(
            &registry,
            Invocation::receiver_ref(&service).arg(String::from("render")),
        )
        .expect("UFCS binding selects a resolved adapter");
    let InvocationOutput::Owned(value) = output else {
        panic!("method returns an owned string")
    };
    assert_eq!(
        DynamicOwned::<Local>::downcast_ref(&value).map(String::as_str),
        Some("render")
    );

    let MethodLookup::Unique(_) = registry
        .effective_view(std::any::TypeId::of::<Service<u8>>())
        .lookup_method(MethodQualifier::Inherent, "pair")
    else {
        panic!("inherent method qualifier resolves the bound impl");
    };

    let mut mutable_service = Service(1_u8);
    let MethodLookup::Unique(replace) = Service::<u8>::type_descriptor().methods_named_in(&registry, "replace") else {
        panic!("mutable receiver method should be visible");
    };
    let _output = replace
        .invoke(&registry, Invocation::receiver_mut(&mut mutable_service).arg(9_u8))
        .expect("mutable receiver is unpacked and passed to the monomorphized method");
    drop(_output);
    assert_eq!(mutable_service.0, 9);

    let MethodLookup::Unique(associated) = Utility::type_descriptor().methods_named_in(&registry, "associated") else {
        panic!("associated function should be visible");
    };
    let output = associated
        .invoke(
            &registry,
            Invocation::associated_function().arg(String::from("associated")),
        )
        .expect("associated function binding has no receiver");
    let InvocationOutput::Owned(value) = output else {
        panic!("associated function returns an owned string")
    };
    assert_eq!(
        DynamicOwned::<Local>::downcast_ref(&value).map(String::as_str),
        Some("associated")
    );

    let MethodLookup::Unique(borrowed) = Utility::type_descriptor().methods_named_in(&registry, "borrowed") else {
        panic!("borrowed output method should be visible");
    };
    let text = String::from("borrowed");
    let output = borrowed
        .invoke(&registry, Invocation::associated_function().arg_ref(&text))
        .expect("borrowed result retains its input lifetime");
    let InvocationOutput::Ref { value, .. } = output else {
        panic!("method returns a borrowed result")
    };
    assert_eq!(value.downcast_ref::<String>().map(String::as_str), Some("borrowed"));

    let MethodLookup::Unique(async_owned) = Utility::type_descriptor().methods_named_in(&registry, "async_owned")
    else {
        panic!("async method should be visible");
    };
    let output = async_owned
        .invoke(&registry, Invocation::associated_function().arg(String::from("future")))
        .expect("async method returns a deferred future");
    assert!(matches!(output, InvocationOutput::Future(_)));

    let MethodLookup::Unique(set_pin) = PinUtility::type_descriptor().methods_named_in(&registry, "set_pin") else {
        panic!("pinned receiver method should be visible");
    };
    let mut pinned = PinUtility(2);
    let mut pinned = unsafe { Pin::new_unchecked(&mut pinned) };
    let output = set_pin
        .invoke_pinned_mut(&registry, Invocation::receiver_pin_mut(pinned.as_mut()).arg(6_u8))
        .expect("pinned adapter preserves the receiver pin");
    drop(output);
    assert_eq!(pinned.as_ref().get_ref().0, 6);
}

#[test]
fn same_named_trait_methods_resolve_by_exact_ufcs_identity() {
    let mut builder = RegistrySnapshotBuilder::new();
    builder.include_linked_fragments();
    builder.add_method_binding(
        bind_method!(<Service<u8> as Render<u8>>::render::<String> as fn(&Service<u8>, String) -> String),
        FragmentIdentity::new("macro-method-test", module_path!(), line!(), 2, "binding", 1),
    );
    builder.add_method_binding(
        bind_method!(<Service<u8> as Paint<u8>>::render::<String> as fn(&Service<u8>, String) -> String),
        FragmentIdentity::new("macro-method-test", module_path!(), line!(), 2, "binding", 2),
    );
    let registry = builder
        .build()
        .expect("UFCS trait identities disambiguate same-named methods");
    let render_id = <Service<u8> as Render<u8>>::__qubit_reflect_trait_payload()
        .definition()
        .trait_id()
        .clone();
    let paint_id = <Service<u8> as Paint<u8>>::__qubit_reflect_trait_payload()
        .definition()
        .trait_id()
        .clone();
    assert_ne!(render_id, paint_id);
    let trait_bindings: Vec<_> = registry
        .method_bindings()
        .iter()
        .filter(|binding| binding.implementation().implemented_trait_id().is_some())
        .collect();
    assert_eq!(trait_bindings.len(), 2);
    assert!(
        trait_bindings
            .iter()
            .any(|binding| binding.implementation().implemented_trait_id() == Some(&render_id))
    );
    assert!(
        trait_bindings
            .iter()
            .any(|binding| binding.implementation().implemented_trait_id() == Some(&paint_id))
    );
}
