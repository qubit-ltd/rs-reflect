// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::capability::CapabilityKey;
use qubit_reflect::context::ReflectRegistry;
use qubit_reflect::descriptor::ReflectedType;
use qubit_reflect::ReflectionContext;
use qubit_reflect::TypeDescriptor;

fn old_require_methods_are_not_public(
    context: &ReflectionContext,
    descriptor: TypeDescriptor<'static>,
) {
    let _ = context.require_type_by_name("fixture::Type");
    let _ = descriptor.require_field("field");
    let _ = descriptor.require_method("method");
}

fn main() {
    let _ = std::mem::size_of::<CapabilityKey<u8>>();
    let _ = std::mem::size_of::<ReflectedType>();
    let _ = std::mem::size_of::<ReflectRegistry>();
    let _ = old_require_methods_are_not_public;
}
