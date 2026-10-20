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
    #[reflect(skip)]
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
    let private = descriptor.field("private").unwrap();

    assert_eq!(
        reflected.access(),
        qubit_reflect::descriptor::FieldAccess::new(true, true, true, true)
    );
    assert!(descriptor.field("opaque").is_err());
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
fn unsafe_dynamic_mutable_access_can_borrow_a_private_field() {
    let descriptor = TypeDescriptor::of::<ThreadSafeAccess>();
    let private = descriptor.field("private").unwrap();
    let mut owner = ThreadSafeAccess { value: 7, private: 9 };

    // SAFETY: This test owns the value and explicitly exercises the unsafe
    // private-field access contract for its own fixture.
    let mut field = unsafe { private.get_dynamic_mut_unsafe(DynamicMut::<ThreadSafe>::new(&mut owner)) }
        .expect("generated mutable adapter is available");
    *field.downcast_mut::<u8>().expect("field type is u8") = 11;
    drop(field);

    assert_eq!(owner.value, 7);
    assert_eq!(owner.private, 11);
}

#[test]
fn unsafe_dynamic_set_can_replace_a_private_field() {
    let descriptor = TypeDescriptor::of::<ThreadSafeAccess>();
    let private = descriptor.field("private").unwrap();
    let mut owner = ThreadSafeAccess { value: 7, private: 9 };

    // SAFETY: This test owns the value and explicitly exercises the unsafe
    // private-field replacement contract for its own fixture.
    unsafe {
        private
            .set_dynamic_unsafe(
                DynamicMut::<ThreadSafe>::new(&mut owner),
                DynamicOwned::<ThreadSafe>::new(11_u8),
            )
            .expect("generated replacement adapter is available");
    }

    assert_eq!(owner.value, 7);
    assert_eq!(owner.private, 11);
}

#[test]
fn field_access_errors_have_stable_debug_display_text() {
    assert_eq!(FieldAccessError::NotReadable.to_string(), "NotReadable");
    assert_eq!(FieldSetError::NotReplaceable.to_string(), "NotReplaceable");
}
