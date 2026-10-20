// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::Reflect;
use qubit_reflect::ReflectionContext;
use qubit_reflect::capability::Capability;
use qubit_reflect::capability::CapabilityRegistration;
use qubit_reflect::capability::CapabilitySpec;

#[Reflect]
struct Target;

struct Marker;

impl CapabilitySpec for Marker {
    type Adapter = ();

    const ID: &'static str = "example.context_escape";
}

fn capability_from_temporary_context<'ctx>() -> Capability<'ctx, Marker> {
    let context = ReflectionContext::builder()
        .include_type::<Target>()
        .register(CapabilityRegistration::for_type::<Target, Marker>(()))
        .build()
        .expect("fixture context builds");
    context.type_of::<Target>().capability::<Marker>().unwrap()
}

fn main() {
    let _capability = capability_from_temporary_context();
}
