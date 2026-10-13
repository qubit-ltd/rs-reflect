// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![cfg(feature = "derive")]

use std::any::Any;
use std::any::TypeId;
use std::future::Future;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;

use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::StructUpdateInput;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::descriptor::AssociatedConstReader;
use qubit_reflect::descriptor::ImplDescriptor;
use qubit_reflect::expression::TypeExpression;
use qubit_reflect::invoke::InvocationOutput;
use qubit_reflect::invoke::ReflectedFuture;
use qubit_reflect::registry::ReflectRegistry;
use qubit_reflect::value::DynamicOwned;
use qubit_reflect::value::Local;
use qubit_reflect::value::ThreadSafe;

#[Reflect]
trait FutureCoverageContract {
    type Item: Reflect;

    const VALUE: u32;

    fn required(&self);

    fn provided(&self) {}
}

#[Reflect]
struct FutureCoverageTarget;

#[ReflectImpl(definition_provider_v2 = __qubit_reflect_trait_definition_FutureCoverageContract)]
impl FutureCoverageContract for FutureCoverageTarget {
    type Item = u32;

    const VALUE: u32 = 37;

    fn required(&self) {}
}

#[Reflect]
struct GenericConstTarget<T>(T);

#[ReflectImpl]
impl<T> GenericConstTarget<T> {
    #[allow(dead_code)]
    const EXTRA: u32 = 43;
}

fn future_coverage_impl() -> &'static ImplDescriptor {
    ReflectRegistry::initialize()
        .expect("linked associated const registration must initialize")
        .implementations(TypeDescriptor::of::<FutureCoverageTarget>().type_id())
        .iter()
        .copied()
        .find(|implementation| {
            implementation
                .implemented_trait()
                .is_some_and(|descriptor| descriptor.rust_name() == "FutureCoverageContract")
        })
        .expect("the associated const implementation must be registered")
}

fn function_reader_value() -> Box<dyn Any> {
    Box::new(41_u16)
}

#[test]
fn associated_const_bindings_read_erased_values_and_preserve_mismatch_details() {
    let implementation = future_coverage_impl();
    let [value] = implementation.associated_consts() else {
        panic!("one associated const binding is registered")
    };

    assert_eq!(
        value
            .read_any()
            .and_then(|value| value.downcast::<u32>().ok())
            .map(|value| *value),
        Some(37)
    );
    let mismatch = value
        .read::<u16>()
        .expect("the generated closure reader is present")
        .expect_err("associated const reading checks the exact concrete type");
    assert_eq!(mismatch.expected(), TypeId::of::<u16>());
    assert_eq!(mismatch.actual(), TypeId::of::<u32>());
}

#[test]
fn impl_definition_exposes_the_declared_associated_const_type() {
    let registry = ReflectRegistry::initialize().expect("linked impl definitions must initialize");
    let definition = registry
        .impl_definitions()
        .iter()
        .copied()
        .find(|definition| {
            definition
                .associated_consts()
                .iter()
                .any(|constant| constant.rust_name() == "EXTRA")
        })
        .expect("the reflected inherent impl definition is registered");
    let [value] = definition.associated_consts() else {
        panic!("the impl definition declares one associated const")
    };

    assert_eq!(value.rust_name(), "EXTRA");
    assert!(matches!(
        value.declared_type(),
        TypeExpression::Concrete(expression)
            if expression.path().last().is_some_and(|segment| segment.as_ref() == "u32")
    ));
}

#[test]
fn trait_definition_exposes_associated_type_and_method_metadata() {
    let registry = ReflectRegistry::initialize().expect("linked trait definitions must initialize");
    let definition = registry
        .trait_definition_by_path(concat!(module_path!(), "::FutureCoverageContract"))
        .expect("the reflected trait definition is registered");

    let [associated_type] = definition.associated_types() else {
        panic!("the trait declares one associated type");
    };
    assert_eq!(associated_type.rust_name(), "Item");
    assert!(associated_type.default().is_none());

    let required = definition
        .methods()
        .iter()
        .find(|method| method.rust_name() == "required")
        .expect("required method metadata is present");
    assert!(!required.has_default());
    let provided = definition
        .methods()
        .iter()
        .find(|method| method.rust_name() == "provided")
        .expect("default method metadata is present");
    assert!(provided.has_default());
}

#[test]
fn unit_struct_descriptor_exposes_fields_and_unit_construction() {
    let descriptor = TypeDescriptor::of::<FutureCoverageTarget>();

    assert!(descriptor.fields().is_empty());
    assert!(descriptor.construct_unit::<FutureCoverageTarget>().is_ok());
    assert!(
        descriptor
            .update_struct::<FutureCoverageTarget>(StructUpdateInput::new(
                FutureCoverageTarget,
                qubit_reflect::NamedConstructionInput::new(),
            ))
            .is_ok()
    );
    let dynamic = DynamicOwned::<Local>::new(FutureCoverageTarget);
    assert!(dynamic.downcast::<FutureCoverageTarget>().is_ok());
}

#[test]
fn function_and_closure_readers_return_fresh_values_with_exact_types() {
    let function = AssociatedConstReader::new(function_reader_value);
    assert_eq!(function.read::<u16>().expect("function reader type matches"), 41);
    let erased = function.read_any();
    assert_eq!(erased.downcast_ref::<u16>(), Some(&41));

    let closure = AssociatedConstReader::from_getter(|| String::from("fresh"));
    assert_eq!(closure.read::<String>().expect("closure reader type matches"), "fresh");
    assert_eq!(
        closure.read_any().downcast_ref::<String>().map(String::as_str),
        Some("fresh")
    );
    let mismatch = closure
        .read::<u16>()
        .expect_err("closure readers enforce the exact requested type");
    assert_eq!(mismatch.expected(), TypeId::of::<u16>());
    assert_eq!(mismatch.actual(), TypeId::of::<String>());
}

#[test]
fn reflected_futures_forward_pending_then_ready_polls_in_both_modes() {
    let mut polls = 0;
    let mut local = ReflectedFuture::<Local>::new(std::future::poll_fn(|context| {
        polls += 1;
        if polls == 1 {
            context.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(InvocationOutput::Owned(DynamicOwned::<Local>::new(9_u8)))
        }
    }));
    let mut context = Context::from_waker(Waker::noop());
    assert!(matches!(Pin::new(&mut local).poll(&mut context), Poll::Pending));
    let Poll::Ready(InvocationOutput::Owned(value)) = Pin::new(&mut local).poll(&mut context) else {
        panic!("the local reflected future returns its ready output")
    };
    assert_eq!(value.downcast_ref::<u8>(), Some(&9));

    let mut thread_safe =
        ReflectedFuture::<ThreadSafe>::new(async { InvocationOutput::Owned(DynamicOwned::<ThreadSafe>::new(11_u16)) });
    let Poll::Ready(InvocationOutput::Owned(value)) = Pin::new(&mut thread_safe).poll(&mut context) else {
        panic!("the thread-safe reflected future returns its ready output")
    };
    assert_eq!(value.downcast_ref::<u16>(), Some(&11));
}
