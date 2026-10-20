// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Public contracts for linked, typed capability registrations.

#![cfg(feature = "derive")]

use qubit_reflect::Reflect;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::capability::CloneCapability;
use qubit_reflect::register_capability;
use qubit_reflect::value::DynamicRef;
use qubit_reflect::value::Local;

#[Reflect]
#[derive(Clone, Debug, Eq, PartialEq)]
struct LinkedCloneValue(u8);

register_capability!(LinkedCloneValue: CloneCapability);

#[test]
fn linked_capability_macro_registers_a_typed_adapter() {
    let descriptor = TypeDescriptor::of::<LinkedCloneValue>();
    let capability = descriptor
        .capability::<CloneCapability>()
        .expect("the linked Clone registration is available");
    let source = LinkedCloneValue(7);
    let cloned = capability
        .adapter()
        .clone_dynamic(DynamicRef::<Local>::new(&source))
        .expect("the registered adapter clones the exact Rust type");
    let cloned = cloned
        .downcast::<LinkedCloneValue>()
        .expect("the dynamic clone keeps its registered concrete type");

    assert_eq!(cloned, source);
}
