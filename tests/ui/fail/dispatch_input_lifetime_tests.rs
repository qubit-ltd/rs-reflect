// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::Invocation;
use qubit_reflect::MethodDescriptor;
use qubit_reflect::invoke::InvocationFailure;
use qubit_reflect::value::Local;

fn escape_failure(
    method: &MethodDescriptor<'_>,
) -> InvocationFailure<'static, Local> {
    let argument = String::from("borrowed");
    match method.invoke(Invocation::<Local>::associated().arg_ref(&argument)) {
        Err(failure) => failure,
        Ok(_) => panic!("the fixture method fails before execution"),
    }
}

fn main() {}
