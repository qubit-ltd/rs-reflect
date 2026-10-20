// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::Invocation;
use qubit_reflect::MethodDescriptor;
use qubit_reflect::value::Local;
use qubit_reflect::value::ThreadSafe;

fn dispatch_thread_safe(method: &MethodDescriptor<'_>, input: Invocation<'_, ThreadSafe>) {
    let _ = method.invoke(input);
}

fn upgrade(method: &MethodDescriptor<'_>, input: Invocation<'_, Local>) {
    dispatch_thread_safe(method, input);
}

fn main() {}
