// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::TypeId;

use qubit_reflect::__private::codegen_v3::descriptor::opaque_root;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::descriptor::Reflect;

struct Wrong;
impl Reflect for Wrong {
    fn type_descriptor() -> &'static TypeDescriptor {
        TypeDescriptor::of::<u32>()
    }
}

struct Right;
static RIGHT: TypeDescriptor = opaque_root::<Right>("Right");
impl Reflect for Right {
    fn type_descriptor() -> &'static TypeDescriptor {
        &RIGHT
    }
}

#[cfg(feature = "derive")]
#[derive(qubit_reflect::Reflect)]
struct Derived;

#[test]
#[should_panic(expected = "Reflect descriptor type mismatch")]
fn wrong_manual_descriptor_is_rejected() {
    let _ = TypeDescriptor::of::<Wrong>();
}

#[test]
fn manual_builtin_and_derived_descriptors_match_their_types() {
    assert_eq!(TypeDescriptor::of::<Right>().type_id(), TypeId::of::<Right>());
    assert_eq!(TypeDescriptor::of::<u32>().type_id(), TypeId::of::<u32>());
    #[cfg(feature = "derive")]
    assert_eq!(TypeDescriptor::of::<Derived>().type_id(), TypeId::of::<Derived>());
}
