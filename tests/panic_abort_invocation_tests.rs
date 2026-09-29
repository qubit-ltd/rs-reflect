// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Abort-on-panic configuration contract for explicit catching descriptors.

#![cfg(panic = "abort")]

use qubit_reflect as reflect;
use qubit_reflect::Reflect;
use qubit_reflect::descriptor::CatchingAvailability;
use qubit_reflect::descriptor::MethodLookup;
use qubit_reflect::descriptor::MethodQualifier;
use qubit_reflect::invoke::Invocation;
use qubit_reflect::invoke::InvocationDispatchMode;
use qubit_reflect::invoke::InvocationDispatchReason;
use qubit_reflect::reflect_impl;
use qubit_reflect::registry::ReflectRegistry;

#[derive(Reflect)]
struct Worker;

#[reflect_impl]
impl Worker {
    #[reflect(catch_unwind)]
    fn marked() {}
}

#[test]
fn test_abort_configuration_reports_requested_catching_as_unavailable() {
    let registry = ReflectRegistry::initialize().expect("generated fragments must validate");
    let implementations = registry.implementations(Worker::type_descriptor().type_id());
    let MethodLookup::Unique(method) =
        reflect::descriptor::ImplDescriptor::lookup_method(implementations, MethodQualifier::Inherent, "marked")
    else {
        panic!("the marked method must be discoverable")
    };
    let adapter = method.adapter().expect("the normal adapter remains available");
    assert_eq!(
        adapter.catching_availability(),
        CatchingAvailability::UnavailablePanicAbort
    );
    let result = method.invoke_catching_local(registry, Invocation::associated([]));
    let Err(unavailable) = result else {
        panic!("abort mode cannot dispatch catching")
    };
    assert_eq!(unavailable.mode(), InvocationDispatchMode::CatchingLocal);
    assert_eq!(unavailable.reason(), &InvocationDispatchReason::PanicAbort);
    assert!(unavailable.into_invocation().arguments().is_empty());
}
