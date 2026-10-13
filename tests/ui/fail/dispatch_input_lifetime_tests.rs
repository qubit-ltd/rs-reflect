// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::Invocation;
use qubit_reflect::InvocationFailure;
use qubit_reflect::ReflectRegistry;
use qubit_reflect::descriptor::MethodInstanceDescriptor;
use qubit_reflect::value::Local;

fn escape_failure(
    method: &MethodInstanceDescriptor,
    registry: &ReflectRegistry,
) -> InvocationFailure<'static, Local> {
    let argument = String::from("borrowed");
    match method.invoke(registry, Invocation::associated_function().arg_ref(&argument)) {
        Err(failure) => failure,
        Ok(_) => panic!("the fixture method fails before execution"),
    }
}

fn main() {}
