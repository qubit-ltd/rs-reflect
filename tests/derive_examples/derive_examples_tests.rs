// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Executable coverage for the three runtime examples in derive macro docs.

use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::TypeDescriptor;

#[Reflect]
struct Account {
    pub id: u32,
    #[reflect(skip)]
    internal: String,
}

/// Checks that an example type exposes its public field and omits skipped data.
#[test]
fn test_reflect_type_example() {
    let descriptor = TypeDescriptor::of::<Account>();
    let account = Account {
        id: 7,
        internal: String::from("private application state"),
    };

    assert!(descriptor.fields().get("id").is_some());
    assert!(descriptor.fields().get("internal").is_none());
    assert_eq!(account.internal, "private application state");
}

#[Reflect]
trait Named {
    fn name(&self) -> &'static str;
}

#[Reflect]
struct NamedService;

#[ReflectImpl]
impl Named for NamedService {
    fn name(&self) -> &'static str {
        "service"
    }
}

/// Checks that the documented trait implementation is reflected.
#[test]
fn test_reflect_trait_example() {
    let implementations = TypeDescriptor::of::<NamedService>()
        .impls()
        .expect("valid trait registrations");
    assert!(implementations.iter().any(|item| item.implemented_trait().is_some()));
}

#[Reflect]
struct InherentService;

#[ReflectImpl]
impl InherentService {
    pub fn ping(&self) {}
}

/// Checks that an inherent implementation is described without policy flags.
#[test]
fn test_reflect_impl_example() {
    InherentService.ping();
    let implementations = TypeDescriptor::of::<InherentService>()
        .impls()
        .expect("valid inherent method registrations");
    assert!(implementations.iter().any(|item| item.method("ping").is_ok()));
}
