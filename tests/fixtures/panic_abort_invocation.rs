// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Executable fixture for the abort-on-panic catching descriptor contract.

use qubit_reflect::Reflect as ReflectDerive;
#[cfg(panic = "abort")]
use qubit_reflect::descriptor;
use qubit_reflect::reflect_impl as reflect_impl_macro;
#[cfg(panic = "abort")]
use qubit_reflect::registry;

#[derive(ReflectDerive)]
#[reflect(crate = qubit_reflect)]
struct Worker;

#[reflect_impl_macro(crate = qubit_reflect)]
impl Worker {
    #[reflect(catch_unwind)]
    fn marked() {}
}

#[cfg(not(panic = "abort"))]
fn main() {}

#[cfg(panic = "abort")]
fn main() {
    let registry = registry::ReflectRegistry::initialize().expect("generated fragments must validate");
    let implementations = registry.implementations(Worker::type_descriptor().type_id());
    let descriptor::MethodLookup::Unique(method) =
        descriptor::ImplDescriptor::lookup_method(implementations, descriptor::MethodQualifier::Inherent, "marked")
    else {
        panic!("the marked method must be discoverable")
    };
    let adapter = method.adapter().expect("the normal adapter remains available");
    assert_eq!(
        adapter.catching_availability(),
        descriptor::CatchingAvailability::UnavailablePanicAbort
    );
}
