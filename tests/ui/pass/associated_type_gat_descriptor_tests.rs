// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::ReflectionContext;

struct Target;

#[qubit_reflect::Reflect]
trait GatContract {
    type Output<'a, T>
    where
        Self: 'a,
        T: 'a;
}

impl GatContract for Target {
    type Output<'a, T>
        = (&'a (), T)
    where
        T: 'a;
}

fn main() {
    let definition = ReflectionContext::global()
        .trait_definition_by_name(concat!(module_path!(), "::GatContract"))
        .expect("linked trait definition is available");
    assert_eq!(
        definition.associated_type("Output").unwrap().name(),
        "Output"
    );
}
