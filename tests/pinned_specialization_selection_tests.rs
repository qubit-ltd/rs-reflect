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
use qubit_reflect::ReflectionContext;
use qubit_reflect::bind_method;
use qubit_reflect::descriptor::MethodArguments;
use qubit_reflect::invoke::InvocationErrorKind;
use qubit_reflect::invoke::PinnedInvocation;
use qubit_reflect::value::Local;

#[Reflect]
struct PinnedOwner;

#[ReflectImpl]
impl PinnedOwner {
    pub fn echo<T>(self: Pin<&mut Self>, value: T) -> T {
        let _ = self;
        value
    }

    pub fn echo_number<T>(self: Pin<&mut Self>, value: u64) -> u64 {
        let _ = self;
        value + std::mem::size_of::<T>() as u64
    }
}

fn echo_type_u32_binding() -> qubit_reflect::__private::codegen_v4::registration::MethodBinding<PinnedOwner> {
    bind_method!(PinnedOwner::echo::<u32> as fn(Pin<&mut PinnedOwner>, u32) -> u32)
}

fn echo_type_u64_binding() -> qubit_reflect::__private::codegen_v4::registration::MethodBinding<PinnedOwner> {
    bind_method!(PinnedOwner::echo::<u64> as fn(Pin<&mut PinnedOwner>, u64) -> u64)
}

fn echo_number_u32_binding() -> qubit_reflect::__private::codegen_v4::registration::MethodBinding<PinnedOwner> {
    bind_method!(PinnedOwner::echo_number::<u32> as fn(Pin<&mut PinnedOwner>, u64) -> u64)
}

fn echo_number_u64_binding() -> qubit_reflect::__private::codegen_v4::registration::MethodBinding<PinnedOwner> {
    bind_method!(PinnedOwner::echo_number::<u64> as fn(Pin<&mut PinnedOwner>, u64) -> u64)
}

#[test]
fn test_selected_pinned_specializations_invoke_and_preserve_rejected_inputs() {
    let context = ReflectionContext::builder()
        .include_linked()
        .bind(echo_type_u32_binding())
        .bind(echo_type_u64_binding())
        .bind(echo_number_u32_binding())
        .bind(echo_number_u64_binding())
        .build()
        .expect("both pinned methods have two registered bindings");
    let method = context
        .type_of::<PinnedOwner>()
        .method("echo_number")
        .expect("the pinned generic method with a concrete parameter is reflected");
    assert_eq!(method.specializations().len(), 2);

    let type_method = context
        .type_of::<PinnedOwner>()
        .method("echo")
        .expect("the generic type-parameter signature is reflected");
    assert_eq!(type_method.specializations().len(), 2);
    let type_specialization = type_method
        .specialization(&MethodArguments::new().type_of::<u32>())
        .expect("the u32 binding is selected by exact type argument");
    let mut generic_argument_owner = Box::pin(PinnedOwner);
    let Err(generic_argument_failure) = type_specialization
        .invoke_pinned(PinnedInvocation::<Local, _>::mutable(Pin::as_mut(&mut generic_argument_owner)).arg(7_u32))
    else {
        panic!("the missing concrete metadata on generic parameter T is reported");
    };
    assert!(matches!(
        generic_argument_failure.error().kind(),
        InvocationErrorKind::ArgumentTypeUnavailable { index: 0 }
    ));
    drop(generic_argument_failure);

    let mut unselected_owner = Box::pin(PinnedOwner);
    let unselected_address = std::ptr::from_ref(Pin::as_ref(&unselected_owner).get_ref());
    let Err(failure) = method.invoke_pinned(PinnedInvocation::<Local, _>::mutable(Pin::as_mut(
        &mut unselected_owner,
    ))) else {
        panic!("multiple specializations require exact selection");
    };
    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::SpecializationSelectionRequired { count: 2 }
    ));
    drop(failure);

    let selected_u32 = method
        .specialization(&MethodArguments::new().type_of::<u32>())
        .expect("u32 selects its exact specialization");
    let mut invalid_owner = Box::pin(PinnedOwner);
    let invalid_address = std::ptr::from_ref(Pin::as_ref(&invalid_owner).get_ref());
    let Err(failure) =
        selected_u32.invoke_pinned(PinnedInvocation::<Local, _>::mutable(Pin::as_mut(&mut invalid_owner)).arg(9_u32))
    else {
        panic!("the selected u32 adapter rejects a u64 argument");
    };
    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::ArgumentTypeMismatch { .. }
    ));
    let Err(failure) = selected_u32.invoke_pinned(failure.into_invocation()) else {
        panic!("recovered pinned receiver and argument remain unchanged");
    };
    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::ArgumentTypeMismatch { .. }
    ));
    drop(failure);
    assert_eq!(
        std::ptr::from_ref(Pin::as_ref(&invalid_owner).get_ref()),
        invalid_address
    );

    let mut owner_u32 = Box::pin(PinnedOwner);
    let output = match selected_u32
        .invoke_pinned(PinnedInvocation::<Local, _>::mutable(Pin::as_mut(&mut owner_u32)).arg(17_u64))
    {
        Ok(output) => output,
        Err(failure) => panic!("selected u32 pinned adapter failed: {:?}", failure.error().kind()),
    };
    assert_eq!(
        output
            .into_owned::<u64>()
            .unwrap_or_else(|_| panic!("echo_number returns u64")),
        21
    );

    let selected_u64 = method
        .specialization(&MethodArguments::new().type_of::<u64>())
        .expect("u64 selects its exact specialization");
    let mut owner_u64 = Box::pin(PinnedOwner);
    let output = match selected_u64
        .invoke_pinned(PinnedInvocation::<Local, _>::mutable(Pin::as_mut(&mut owner_u64)).arg(23_u64))
    {
        Ok(output) => output,
        Err(_) => panic!("selected u64 pinned adapter is callable"),
    };
    assert_eq!(
        output
            .into_owned::<u64>()
            .unwrap_or_else(|_| panic!("echo_number returns u64")),
        31
    );

    assert_eq!(
        std::ptr::from_ref(Pin::as_ref(&unselected_owner).get_ref()),
        unselected_address
    );
}
