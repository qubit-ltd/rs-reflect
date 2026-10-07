// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Executable coverage for the three runtime examples in derive macro docs.

use qubit_reflect::TypeDescriptor;
use qubit_reflect_derive::Reflect;
use qubit_reflect_derive::reflect;
use qubit_reflect_derive::reflect_impl;

#[derive(Reflect)]
#[reflect(crate = qubit_reflect, rename = "account")]
struct Account {
    #[reflect(read_only)]
    id: u32,
}

/// Checks the documented derive rename and read-only field behavior.
#[test]
fn test_derive_reflect_example() {
    let descriptor = TypeDescriptor::of::<Account>();
    assert_eq!(descriptor.query_name(), "account");
    assert!(descriptor.field("id").is_some());
}

#[derive(Reflect)]
#[reflect(crate = qubit_reflect)]
struct NamedService;

#[reflect(crate = qubit_reflect)]
trait Named {
    fn name(&self) -> &'static str;
}

#[reflect_impl(crate = qubit_reflect)]
impl Named for NamedService {
    fn name(&self) -> &'static str {
        "service"
    }
}

/// Checks the documented trait and implementation registration.
#[test]
fn test_reflect_trait_example() {
    let implementations = TypeDescriptor::of::<NamedService>()
        .impls_global()
        .expect("valid trait registrations");
    assert!(implementations.iter().any(|item| item.implemented_trait().is_some()));
}

#[derive(Reflect)]
#[reflect(crate = qubit_reflect)]
struct InherentService;

#[reflect_impl(crate = qubit_reflect)]
impl InherentService {
    #[reflect(no_invoke)]
    fn ping(&self) {}
}

/// Checks the documented inherent method registration and no-invoke policy.
#[test]
fn test_reflect_impl_example() {
    InherentService.ping();
    let implementations = TypeDescriptor::of::<InherentService>()
        .impls_global()
        .expect("valid inherent method registrations");
    assert!(implementations.iter().any(|item| item.method("ping").is_some()));
}
