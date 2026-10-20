// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::Invocation;
use qubit_reflect::InvocationOutput;
use qubit_reflect::MethodDescriptor;
use qubit_reflect::value::Local;

fn escape_receiver(
    method: &MethodDescriptor<'_>,
) -> InvocationOutput<'static, Local> {
    let receiver = String::from("borrowed receiver");
    match method.invoke(Invocation::<Local>::on_ref(&receiver)) {
        Ok(output) => output,
        Err(_) => panic!("the fixture method is callable"),
    }
}

fn escape_argument(
    method: &MethodDescriptor<'_>,
) -> InvocationOutput<'static, Local> {
    let argument = String::from("borrowed argument");
    match method.invoke(Invocation::<Local>::associated().arg_ref(&argument)) {
        Ok(output) => output,
        Err(_) => panic!("the fixture method is callable"),
    }
}

fn main() {}
