// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Integration coverage for callable invocation adapter descriptors.
use qubit_reflect::ReflectRegistry;
use qubit_reflect::descriptor::InvocationAdapter;
use qubit_reflect::invoke::Invocation;
use qubit_reflect::invoke::InvocationFailure;
use qubit_reflect::invoke::InvocationOutput;
use qubit_reflect::value::DynamicOwned;
use qubit_reflect::value::Local;

/// Returns one owned output through the local invocation boundary.
fn return_seven<'call>(
    _registry: &ReflectRegistry,
    _invocation: Invocation<'call, Local>,
) -> Result<InvocationOutput<'call, Local>, InvocationFailure<'call, Local>> {
    Ok(InvocationOutput::Owned(DynamicOwned::<Local>::new(7_u8)))
}

#[test]
fn test_invocation_adapter_local_descriptor_invokes_its_typed_entry_point() {
    let adapter = InvocationAdapter::local(return_seven);
    let output = adapter
        .invoke_local(
            ReflectRegistry::initialize().expect("valid fixture registry"),
            Invocation::associated([]),
        )
        .expect("the local entry point must be present")
        .expect("the local adapter must be callable");
    let InvocationOutput::Owned(value) = output else {
        panic!("adapter must retain its owned output");
    };
    let Ok(value) = DynamicOwned::<Local>::downcast::<u8>(value) else {
        panic!("output type must be u8");
    };
    assert_eq!(value, 7);
}
