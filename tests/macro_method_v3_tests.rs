// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![cfg(feature = "derive")]

use qubit_reflect::Invocation;
use qubit_reflect::InvocationOutput;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::ReflectionContext;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::bind_method;
use qubit_reflect::lookup::Lookup;
use qubit_reflect::register_method_binding;

#[Reflect]
struct Service(u8);

#[Reflect]
struct Utility;

impl Service {
    pub fn pair<U: ToString>(&self, value: U) -> String {
        value.to_string()
    }

    pub fn const_method<const N: usize>(&self, value: String) -> usize {
        value.len() + N
    }

    pub fn replace(&mut self, value: u8) {
        self.0 = value;
    }

    pub fn echo(&self, value: String) -> String {
        value
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

#[Reflect]
trait Render<T> {
    fn render(&self, value: String) -> String;
}

#[Reflect]
trait Paint<T> {
    fn render(&self, value: String) -> String;
}

#[ReflectImpl(reflected_trait = Render::<u8>)]
impl Render<u8> for Service {
    fn render(&self, value: String) -> String {
        value
    }
}

#[ReflectImpl(reflected_trait = Paint::<u8>)]
impl Paint<u8> for Service {
    fn render(&self, value: String) -> String {
        format!("paint:{value}")
    }
}

register_method_binding!(Service::echo as fn(&Service, String) -> String);

fn context() -> ReflectionContext {
    ReflectionContext::builder()
        .include_linked()
        .build()
        .expect("linked method bindings and declarations must form a valid context")
}

fn owned_string(output: InvocationOutput<'_, qubit_reflect::value::Local>) -> String {
    let InvocationOutput::Owned(value) = output else {
        panic!("method returns an owned string")
    };
    value
        .downcast::<String>()
        .unwrap_or_else(|_| panic!("method output retains its String type"))
}

#[test]
fn test_registered_method_binding_invokes_through_context_bound_descriptor() {
    let context = context();
    let target = context.type_of::<Service>();
    let service = Service(7);

    let echo = target.method("echo").expect("registered method resolves uniquely");
    assert_eq!(
        owned_string(
            echo.invoke(Invocation::<qubit_reflect::value::Local>::on_ref(&service).arg(String::from("echo")))
                .unwrap_or_else(|failure| panic!("method binding failed: {:?}", failure.error().kind())),
        ),
        "echo"
    );
}

#[test]
fn test_same_named_trait_bindings_remain_distinct_in_method_lookup() {
    let context = context();
    let target = context.type_of::<Service>();
    let service = Service(7);
    let Lookup::Ambiguous(methods) = target.lookup_method("render").expect("linked trait methods resolve") else {
        panic!("same-named methods from separate traits remain separate candidates")
    };
    assert_eq!(methods.len(), 2);

    let outputs = methods
        .iter()
        .map(|method| {
            owned_string(
                method
                    .invoke(Invocation::<qubit_reflect::value::Local>::on_ref(&service).arg(String::from("render")))
                    .unwrap_or_else(|failure| panic!("UFCS binding failed: {:?}", failure.error().kind())),
            )
        })
        .collect::<Vec<_>>();
    assert!(outputs.iter().any(|output| output == "render"));
    assert!(outputs.iter().any(|output| output == "paint:render"));
}

#[test]
fn test_bind_method_supports_receiver_and_async_signatures() {
    let _generic_type = bind_method!(Service::pair::<String> as fn(&Service, String) -> String);
    let _generic_const = bind_method!(Service::const_method::<3> as fn(&Service, String) -> usize);
    let _mutable = bind_method!(Service::replace as fn(&mut Service, u8));
    let _associated = bind_method!(Utility::associated as fn(String) -> String);
    let _borrowed = bind_method!(Utility::borrowed as fn(&String) -> &String);
    let _asynchronous = bind_method!(Utility::async_owned as async fn(String) -> String);
    let _binding = bind_method!(<Service as Render<u8>>::render as fn(&Service, String) -> String);
    let _descriptor = TypeDescriptor::of::<Utility>();
}
