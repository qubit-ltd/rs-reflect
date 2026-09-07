// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::Invocation;
use qubit_reflect::ReflectRegistry;
use qubit_reflect::descriptor::MethodInstanceDescriptor;
use qubit_reflect::value::Local;

fn upgrade(method: &MethodInstanceDescriptor, registry: &ReflectRegistry, input: Invocation<'_, Local>) {
    let _ = method.invoke_thread_safe(registry, input);
}

fn main() {}
