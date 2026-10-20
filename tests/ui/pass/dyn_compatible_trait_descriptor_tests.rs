// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::ReflectionContext;
use qubit_reflect::reflect_external_trait;
use qubit_reflect::register_trait_object;

reflect_external_trait! {
    pub ExternalDebug = std::fmt::Debug;
}

register_trait_object!(dyn std::fmt::Debug);

fn main() {
    let context = ReflectionContext::global();
    let definition = context
        .external_trait_definition("std::fmt::Debug")
        .expect("the linked external trait registration validates")
        .expect("the external trait identity is registered");
    assert_eq!(definition.path(), "std::fmt::Debug");

    let object = context
        .trait_object::<dyn std::fmt::Debug>()
        .expect("the linked trait-object registration validates")
        .expect("the trait object is registered");
    assert_eq!(object.trait_path(), "dyn std :: fmt :: Debug");
}
