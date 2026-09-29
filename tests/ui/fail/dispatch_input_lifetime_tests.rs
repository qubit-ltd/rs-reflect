// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::descriptor::InvocationAdapter;
use qubit_reflect::invoke::Invocation;
use qubit_reflect::invoke::InvocationArg;
use qubit_reflect::invoke::InvocationUnavailable;
use qubit_reflect::registry::ReflectRegistry;
use qubit_reflect::value::Local;
use qubit_reflect::ReflectedRef;

fn opaque_entry() {}

fn escape_input(registry: &ReflectRegistry) -> InvocationUnavailable<Invocation<'static, Local>> {
    let argument = String::from("borrowed");
    let input = Invocation::associated([InvocationArg::Ref(ReflectedRef::new(&argument))]);
    match InvocationAdapter::new(opaque_entry).invoke_local(registry, input) {
        Err(unavailable) => unavailable,
        Ok(_) => panic!("opaque entry"),
    }
}

fn main() {}
