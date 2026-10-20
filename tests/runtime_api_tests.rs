// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Runtime-only API tests that remain discoverable without the derive feature.

use qubit_reflect::TypeDescriptor;
use qubit_reflect::invoke::Invocation;
use qubit_reflect::value::DynamicOwned;
use qubit_reflect::value::Local;

/// Verifies core descriptors and dynamic values remain usable without macros.
#[test]
fn test_runtime_only_descriptor_and_value_path() {
    let descriptor = TypeDescriptor::of::<u32>();
    assert_eq!(descriptor.rust_type_name(), "u32");
    assert_eq!(descriptor.short_name(), "u32");

    let value = DynamicOwned::<Local>::new(17_u32);
    assert_eq!(value.downcast_ref::<u32>(), Some(&17));
}

/// Verifies invocation input construction is part of the runtime-only surface.
#[test]
fn test_runtime_only_associated_invocation_path() {
    let _invocation = Invocation::<Local>::associated().arg(17_u32);
}
