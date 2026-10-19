// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0 (the "License");
//    you may not use this file except in compliance with the License.
// =============================================================================

//! Integration tests for concrete generic type identity and field navigation.

use qubit_reflect::Reflect;
use qubit_reflect::TypeDescriptor;

#[Reflect]
struct GenericBox<T> {
    value: T,
}

#[test]
fn test_concrete_generic_descriptor_resolves_reflected_field_type() {
    let descriptor = TypeDescriptor::of::<GenericBox<u32>>();
    let field = descriptor.field("value").expect("generic field exists");

    assert_eq!(field.find_concrete_type().unwrap().type_id(), std::any::TypeId::of::<u32>());
}
