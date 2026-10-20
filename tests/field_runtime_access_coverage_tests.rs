// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![cfg(feature = "derive")]

use qubit_reflect::Reflect;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::descriptor::FieldAccessError;
use qubit_reflect::descriptor::FieldSetError;
use qubit_reflect::value::DynamicMut;
use qubit_reflect::value::DynamicOwned;
use qubit_reflect::value::ThreadSafe;

struct OpaqueField(&'static str);

#[Reflect]
struct AccessMatrix {
    pub reflected: u32,
    pub opaque: OpaqueField,
    private: u8,
}

#[Reflect]
struct ThreadSafeAccess {
    pub value: u32,
    private: u8,
}

#[test]
fn access_capabilities_follow_available_dynamic_adapters() {
    let descriptor = TypeDescriptor::of::<AccessMatrix>();
    let reflected = descriptor.field("reflected").unwrap();
    let opaque = descriptor.field("opaque").unwrap();
    let private = descriptor.field("private").unwrap();

    assert_eq!(
        reflected.access(),
        qubit_reflect::descriptor::FieldAccess::new(true, true, true, true)
    );
    assert_eq!(
        opaque.access(),
        qubit_reflect::descriptor::FieldAccess::new(false, false, false, false)
    );
    assert_eq!(
        private.access(),
        qubit_reflect::descriptor::FieldAccess::new(true, true, true, true)
    );

    let owner = AccessMatrix {
        reflected: 3,
        opaque: OpaqueField("kept"),
        private: 4,
    };
    assert_eq!(owner.opaque.0, "kept");
    assert_eq!(owner.private, 4);
}

#[test]
fn thread_safe_private_write_retains_dynamic_replacement() {
    let descriptor = TypeDescriptor::of::<ThreadSafeAccess>();
    let private = descriptor.field("private").unwrap();
    let mut owner = ThreadSafeAccess { value: 7, private: 9 };

    let failure = private
        .set_dynamic(
            DynamicMut::<ThreadSafe>::new(&mut owner),
            DynamicOwned::<ThreadSafe>::new(11_u8),
        )
        .expect_err("dynamic safe access must honor source visibility");
    assert!(matches!(failure.error(), FieldSetError::NotPublic));
    assert_eq!(failure.replacement().downcast_ref::<u8>(), Some(&11));
    let (error, replacement) = failure.into_parts();
    assert!(matches!(error, FieldSetError::NotPublic));
    assert_eq!(replacement.downcast::<u8>().unwrap(), 11);
    assert_eq!(owner.value, 7);
    assert_eq!(owner.private, 9);
}

#[test]
fn field_access_errors_have_stable_debug_display_text() {
    assert_eq!(FieldAccessError::NotReadable.to_string(), "NotReadable");
    assert_eq!(FieldSetError::NotReplaceable.to_string(), "NotReplaceable");
}
