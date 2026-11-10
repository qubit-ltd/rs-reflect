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
use qubit_reflect::value::ThreadSafe;

#[Reflect]
struct PinnedOwner;

#[ReflectImpl]
impl PinnedOwner {
    pub fn echo<T>(self: Pin<&mut Self>, value: T) -> T {
        let _ = self;
        value
    }

    pub fn echo_shared<T>(self: Pin<&Self>, value: T) -> T {
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

fn echo_shared_u32_binding() -> qubit_reflect::__private::codegen_v4::registration::MethodBinding<PinnedOwner> {
    bind_method!(PinnedOwner::echo_shared::<u32> as fn(Pin<&PinnedOwner>, u32) -> u32)
}

fn echo_thread_safe_u32_binding() -> qubit_reflect::__private::codegen_v4::registration::MethodBinding<PinnedOwner> {
    bind_method!(PinnedOwner::echo::<u32> as thread_safe fn(Pin<&mut PinnedOwner>, u32) -> u32)
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
    let output = type_specialization
        .invoke_pinned(PinnedInvocation::<Local, _>::mutable(Pin::as_mut(&mut generic_argument_owner)).arg(7_u32))
        .unwrap_or_else(|failure| panic!("selected u32 pinned adapter failed: {:?}", failure.error().kind()));
    assert_eq!(output.into_owned::<u32>().ok(), Some(7));

    let type_specialization_u64 = type_method
        .specialization(&MethodArguments::new().type_of::<u64>())
        .expect("the u64 binding is selected by exact type argument");
    let mut generic_argument_owner_u64 = Box::pin(PinnedOwner);
    let output = type_specialization_u64
        .invoke_pinned(PinnedInvocation::<Local, _>::mutable(Pin::as_mut(&mut generic_argument_owner_u64)).arg(9_u64))
        .unwrap_or_else(|failure| panic!("selected u64 pinned adapter failed: {:?}", failure.error().kind()));
    assert_eq!(output.into_owned::<u64>().ok(), Some(9));

    let mut named_owner = Box::pin(PinnedOwner);
    let output = type_specialization_u64
        .invoke_pinned(PinnedInvocation::<Local, _>::mutable(Pin::as_mut(&mut named_owner)).named_arg("value", 13_u64))
        .unwrap_or_else(|failure| panic!("named pinned argument failed: {:?}", failure.error().kind()));
    assert_eq!(output.into_owned::<u64>().ok(), Some(13));

    let mut wrong_type_owner = Box::pin(PinnedOwner);
    let wrong_type_address = std::ptr::from_ref(Pin::as_ref(&wrong_type_owner).get_ref());
    let Err(failure) = type_specialization
        .invoke_pinned(PinnedInvocation::<Local, _>::mutable(Pin::as_mut(&mut wrong_type_owner)).arg(11_u64))
    else {
        panic!("the selected u32 signature rejects a u64 argument");
    };
    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::ArgumentTypeMismatch { index: 0, .. }
    ));
    drop(failure);
    assert_eq!(
        std::ptr::from_ref(Pin::as_ref(&wrong_type_owner).get_ref()),
        wrong_type_address
    );

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

    let shared_argument = 37_u32;
    let shared_argument_address = std::ptr::from_ref(&shared_argument);
    let mut shared_mode_owner = Box::pin(PinnedOwner);
    let shared_mode_receiver_address = std::ptr::from_ref(Pin::as_ref(&shared_mode_owner).get_ref());
    let Err(failure) = selected_u32.invoke_pinned(
        PinnedInvocation::<Local, _>::mutable(Pin::as_mut(&mut shared_mode_owner)).arg_ref(&shared_argument),
    ) else {
        panic!("a shared reference does not match the by-value u32 parameter");
    };
    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::ArgumentModeMismatch { index: 0 }
    ));
    let Err(failure) = selected_u32.invoke_pinned(failure.into_invocation()) else {
        panic!("recovered pinned receiver and shared argument remain unchanged");
    };
    assert!(matches!(
        failure.error().kind(),
        InvocationErrorKind::ArgumentModeMismatch { index: 0 }
    ));
    drop(failure);
    assert_eq!(std::ptr::from_ref(&shared_argument), shared_argument_address);
    assert_eq!(shared_argument, 37);
    assert_eq!(
        std::ptr::from_ref(Pin::as_ref(&shared_mode_owner).get_ref()),
        shared_mode_receiver_address
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

#[test]
fn test_sole_pinned_specialization_and_context_metadata_are_isolated() {
    let u32_context = ReflectionContext::builder()
        .include_linked()
        .bind(echo_type_u32_binding())
        .build()
        .expect("u32 binding registers independently");
    let u64_context = ReflectionContext::builder()
        .include_linked()
        .bind(echo_type_u64_binding())
        .build()
        .expect("u64 binding registers independently");

    let method = u32_context
        .type_of::<PinnedOwner>()
        .method("echo")
        .expect("the context contains the bound method");
    assert_eq!(method.specializations().len(), 1);
    let mut owner = Box::pin(PinnedOwner);
    let output = method
        .invoke_pinned(PinnedInvocation::<Local, _>::mutable(Pin::as_mut(&mut owner)).arg(19_u32))
        .unwrap_or_else(|failure| panic!("sole u32 specialization failed: {:?}", failure.error().kind()));
    assert_eq!(output.into_owned::<u32>().ok(), Some(19));

    let method = u64_context
        .type_of::<PinnedOwner>()
        .method("echo")
        .expect("the context contains the bound method");
    assert_eq!(method.specializations().len(), 1);
    let mut owner = Box::pin(PinnedOwner);
    let output = method
        .invoke_pinned(PinnedInvocation::<Local, _>::mutable(Pin::as_mut(&mut owner)).arg(23_u64))
        .unwrap_or_else(|failure| panic!("sole u64 specialization failed: {:?}", failure.error().kind()));
    assert_eq!(output.into_owned::<u64>().ok(), Some(23));

    let shared_context = ReflectionContext::builder()
        .include_linked()
        .bind(echo_shared_u32_binding())
        .build()
        .expect("shared pinned binding registers");
    let shared_method = shared_context
        .type_of::<PinnedOwner>()
        .method("echo_shared")
        .expect("the shared pinned method is reflected");
    let owner = Box::pin(PinnedOwner);
    let output = shared_method
        .invoke_pinned(PinnedInvocation::<Local, _>::shared(Pin::as_ref(&owner)).arg(29_u32))
        .unwrap_or_else(|failure| panic!("shared pinned specialization failed: {:?}", failure.error().kind()));
    assert_eq!(output.into_owned::<u32>().ok(), Some(29));

    let thread_safe_context = ReflectionContext::builder()
        .include_linked()
        .bind(echo_thread_safe_u32_binding())
        .build()
        .expect("thread-safe pinned binding registers for a Send + Sync target");
    let thread_safe_method = thread_safe_context
        .type_of::<PinnedOwner>()
        .method("echo")
        .expect("the thread-safe pinned method is reflected");
    let mut owner = Box::pin(PinnedOwner);
    let output = thread_safe_method
        .invoke_pinned(PinnedInvocation::<ThreadSafe, _>::mutable(Pin::as_mut(&mut owner)).arg(31_u32))
        .unwrap_or_else(|failure| panic!("thread-safe pinned specialization failed: {:?}", failure.error().kind()));
    assert_eq!(output.into_owned::<u32>().ok(), Some(31));
}
