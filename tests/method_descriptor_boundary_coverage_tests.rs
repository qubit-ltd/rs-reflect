// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![cfg(feature = "derive")]

use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::ReflectionContext;
use qubit_reflect::bind_method;
use qubit_reflect::descriptor::MethodArguments;
use qubit_reflect::invoke::Invocation;
use qubit_reflect::invoke::InvocationErrorKind;
use qubit_reflect::value::Local;
use qubit_reflect::value::ThreadSafe;

#[Reflect]
struct MethodBoundaryTarget;

#[ReflectImpl]
impl MethodBoundaryTarget {
    pub fn echo<T>(&self, value: T) -> T {
        value
    }

    pub fn mode<T>(&self) -> u32 {
        let _ = std::marker::PhantomData::<T>;
        11
    }
}

fn thread_safe_echo_binding() -> qubit_reflect::__private::codegen_v4::registration::MethodBinding<MethodBoundaryTarget>
{
    bind_method!(MethodBoundaryTarget::echo::<u32> as thread_safe fn(&MethodBoundaryTarget, u32) -> u32)
}

fn thread_safe_mode_binding() -> qubit_reflect::__private::codegen_v4::registration::MethodBinding<MethodBoundaryTarget>
{
    bind_method!(MethodBoundaryTarget::mode::<u32> as thread_safe fn(&MethodBoundaryTarget) -> u32)
}

#[test]
fn generic_method_descriptor_exposes_parameters_and_specialization_selection() {
    let context = ReflectionContext::builder()
        .include_linked()
        .bind(thread_safe_echo_binding())
        .bind(thread_safe_mode_binding())
        .build()
        .expect("the concrete method specialization is registered");
    let descriptor = context
        .type_of::<MethodBoundaryTarget>()
        .method("echo")
        .expect("the generic method is reflected");

    assert_eq!(descriptor.context().id(), context.id());
    assert_eq!(descriptor.parameters().context().id(), context.id());
    assert_eq!(descriptor.parameters().len(), 1);
    assert!(!descriptor.parameters().is_empty());
    let parameter = descriptor.parameters().get("value").unwrap();
    assert_eq!(parameter.context().id(), context.id());
    assert_eq!(parameter.index(), 0);
    assert_eq!(parameter.name(), Some("value"));
    assert!(parameter.type_expression().display().to_string().contains("T"));
    assert_eq!(descriptor.generics().parameters().len(), 1);
    assert_eq!(descriptor.generics().parameter("T").unwrap().name(), "T");
    assert_eq!(descriptor.specializations().len(), 1);
    assert!(!descriptor.specializations().is_empty());
    assert_eq!(descriptor.specializations().iter().len(), 1);

    let specialization = descriptor
        .specialization(&MethodArguments::new().type_of::<u32>())
        .expect("the exact type argument selects the specialization");
    assert_eq!(specialization.method().name(), "echo");
    assert_eq!(specialization.arguments().len(), 1);
    assert!(matches!(
        specialization.availability(),
        qubit_reflect::invoke::InvocationAvailability::Callable
    ));

    let Err(missing) = descriptor.specialization(&MethodArguments::new().type_of::<u64>()) else {
        panic!("a different type argument must not select this specialization");
    };
    assert!(matches!(
        missing,
        qubit_reflect::error::LookupError::Missing {
            subject: qubit_reflect::error::LookupSubject::MethodSpecialization,
            ..
        }
    ));
}

#[test]
fn specialization_invocation_reports_unavailable_value_mode_and_runs_supported_mode() {
    let context = ReflectionContext::builder()
        .include_linked()
        .bind(thread_safe_echo_binding())
        .bind(thread_safe_mode_binding())
        .build()
        .expect("the concrete method specialization is registered");
    let descriptor = context
        .type_of::<MethodBoundaryTarget>()
        .method("mode")
        .expect("the generic method is reflected");
    let target = MethodBoundaryTarget;
    let specialization = descriptor
        .specialization(&MethodArguments::new().type_of::<u32>())
        .expect("the exact type argument selects the specialization");

    let Err(local_failure) = specialization.clone().invoke(Invocation::<Local>::on_ref(&target)) else {
        panic!("a thread-safe-only binding has no local adapter");
    };
    assert_eq!(local_failure.error().kind(), &InvocationErrorKind::ValueModeUnavailable);

    let output = match specialization.invoke(Invocation::<ThreadSafe>::on_ref(&target)) {
        Ok(output) => output,
        Err(_) => panic!("the thread-safe adapter is available"),
    };
    let value = match output.into_owned::<u32>() {
        Ok(value) => value,
        Err(_) => panic!("the adapter returns the declared u32 value"),
    };
    assert_eq!(value, 11);
}
