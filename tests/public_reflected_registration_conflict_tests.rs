// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Tests that ambiguous registration names remain explicit lookup failures.

#![cfg(feature = "derive")]

use qubit_reflect::{Reflect, ReflectionContext};

mod first {
    use qubit_reflect::Reflect;

    #[Reflect]
    pub struct Duplicate;
}

mod second {
    use qubit_reflect::Reflect;

    #[Reflect]
    pub struct Duplicate;
}

#[test]
fn short_name_conflict_does_not_pick_a_registration() {
    let context = ReflectionContext::builder()
        .include_type::<first::Duplicate>()
        .include_type::<second::Duplicate>()
        .build()
        .expect("different Rust types may share a short name");

    assert!(context.type_by_short_name("Duplicate").is_err());
    assert!(
        context
            .type_by_name(std::any::type_name::<first::Duplicate>())
            .is_ok()
    );
    assert!(
        context
            .type_by_name(std::any::type_name::<second::Duplicate>())
            .is_ok()
    );
}
