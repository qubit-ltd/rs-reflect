// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow explicit-imports
//! Integration tests for reflected enum variant access.
use std::any::TypeId;

use qubit_reflect as reflect;
use reflect::__private::descriptor;
use reflect::access::FieldAccessError;
use reflect::access::FieldAccessPolicy;
use reflect::access::FieldIdentity;
use reflect::access::FieldVisibility;
use reflect::descriptor::FieldDescriptor;
use reflect::descriptor::OpaqueTypeDescriptor;
use reflect::descriptor::TypeDescriptor;
use reflect::descriptor::TypeRef;
use reflect::descriptor::VariantDescriptor;
use reflect::descriptor::VariantKind;
use reflect::error::TypeMismatch;
use reflect::identity::Visibility;
use reflect::value::ReflectedRef;

enum Event {
    Progress(u32),
    Done(u32),
}

struct NotAnEvent;

/// Returns the enum root used by its variants and fields.
fn event_descriptor() -> &'static TypeDescriptor {
    &EVENT_DESCRIPTOR
}

/// Tests whether `Progress` is the active variant.
fn is_progress(value: ReflectedRef<'_>) -> Result<bool, TypeMismatch> {
    let event = value
        .downcast_ref::<Event>()
        .expect("the descriptor must validate the active-test target type");
    Ok(matches!(event, Event::Progress(_)))
}

/// Tests whether `Done` is the active variant.
fn is_done(value: ReflectedRef<'_>) -> Result<bool, TypeMismatch> {
    let event = value
        .downcast_ref::<Event>()
        .expect("the descriptor must validate the active-test target type");
    Ok(matches!(event, Event::Done(_)))
}

/// Reads the payload only while `Progress` is active.
fn get_progress_value<'a>(target: ReflectedRef<'a>) -> Result<ReflectedRef<'a>, FieldAccessError> {
    let event = target
        .downcast::<Event>()
        .unwrap_or_else(|_| panic!("the descriptor must validate the field target type"));
    match event {
        Event::Progress(value) => Ok(ReflectedRef::new(value)),
        Event::Done(_) => Err(FieldAccessError::inactive_variant(
            FieldIdentity::new_variant(TypeId::of::<Event>(), "variant_tests::Event", 0, None, 0, "Progress"),
            0,
            "Progress",
        )),
    }
}

/// Reads the payload only while `Done` is active.
fn get_done_value<'a>(target: ReflectedRef<'a>) -> Result<ReflectedRef<'a>, FieldAccessError> {
    let event = target
        .downcast::<Event>()
        .unwrap_or_else(|_| panic!("the descriptor must validate the field target type"));
    match event {
        Event::Done(value) => Ok(ReflectedRef::new(value)),
        Event::Progress(_) => Err(FieldAccessError::inactive_variant(
            FieldIdentity::new_variant(TypeId::of::<Event>(), "variant_tests::Event", 0, None, 1, "Done"),
            1,
            "Done",
        )),
    }
}

static U32_TYPE: OpaqueTypeDescriptor = descriptor::opaque_member::<u32>();
static U32_TYPE_REF: TypeRef = TypeRef::Opaque(&U32_TYPE);
static PROGRESS_FIELDS: [FieldDescriptor; 1] =
    [
        descriptor::field(event_descriptor, 0, None, None, &U32_TYPE_REF, Visibility::Private)
            .with_access(FieldAccessPolicy::ReadWrite, Some(get_progress_value), None, None)
            .with_variant(0, "Progress"),
    ];
static DONE_FIELDS: [FieldDescriptor; 1] =
    [
        descriptor::field(event_descriptor, 0, None, None, &U32_TYPE_REF, Visibility::Private)
            .with_access(FieldAccessPolicy::ReadWrite, Some(get_done_value), None, None)
            .with_variant(1, "Done"),
    ];
static EVENT_VARIANTS: [VariantDescriptor; 2] = [
    descriptor::variant(
        event_descriptor,
        0,
        "Progress",
        "Progress",
        VariantKind::Tuple,
        &PROGRESS_FIELDS,
        is_progress,
    ),
    descriptor::variant(
        event_descriptor,
        1,
        "Done",
        "Done",
        VariantKind::Tuple,
        &DONE_FIELDS,
        is_done,
    ),
];
static EVENT_DESCRIPTOR: TypeDescriptor = descriptor::enum_type::<Event>("variant_tests::Event", &EVENT_VARIANTS);

/// Verifies active testing is checked against the enum root and distinguishes
/// every declared variant.
#[test]
fn test_variant_descriptor_checks_active_variant_and_target_type() {
    let progress = &EVENT_VARIANTS[0];
    let done = &EVENT_VARIANTS[1];
    let event = Event::Progress(5);
    let done_event = Event::Done(9);

    assert_eq!(progress.is_active(ReflectedRef::new(&event)), Ok(true));
    assert_eq!(done.is_active(ReflectedRef::new(&event)), Ok(false));
    assert_eq!(done.is_active(ReflectedRef::new(&done_event)), Ok(true));
    assert_eq!(
        DONE_FIELDS[0]
            .get(ReflectedRef::new(&done_event))
            .expect("the active Done payload should be readable")
            .downcast_ref::<u32>(),
        Some(&9)
    );

    let error = progress
        .is_active(ReflectedRef::new(&NotAnEvent))
        .expect_err("a non-enum target must be rejected before the adapter");
    assert_eq!(error.expected(), TypeId::of::<Event>());
    assert_eq!(error.actual(), TypeId::of::<NotAnEvent>());
}

/// Verifies a field adapter reports a structured error when its containing
/// variant is inactive instead of reading invalid enum storage.
#[test]
fn test_variant_field_rejects_inactive_variant_without_panicking() {
    let field = &PROGRESS_FIELDS[0];
    let event = Event::Done(9);

    let error = match field.get(ReflectedRef::new(&event)) {
        Ok(_) => panic!("an inactive variant payload must not be read"),
        Err(error) => error,
    };
    assert!(matches!(
        error,
        FieldAccessError::InactiveVariant {
            variant_index: 0,
            variant_rust_name: "Progress",
            ..
        }
    ));
}

/// Verifies fields at the same position in different variants retain distinct
/// identities in descriptor-level errors.
#[test]
fn test_variant_fields_include_variant_in_runtime_identity() {
    let other = NotAnEvent;
    let progress_error = match PROGRESS_FIELDS[0].get(ReflectedRef::new(&other)) {
        Ok(_) => panic!("the wrong target type must be rejected"),
        Err(error) => error,
    };
    let done_error = match DONE_FIELDS[0].get(ReflectedRef::new(&other)) {
        Ok(_) => panic!("the wrong target type must be rejected"),
        Err(error) => error,
    };

    assert_ne!(progress_error.field(), done_error.field());
    assert_eq!(progress_error.field().variant_index(), Some(0));
    assert_eq!(progress_error.field().variant_rust_name(), Some("Progress"));
    assert_eq!(done_error.field().variant_index(), Some(1));
    assert_eq!(done_error.field().variant_rust_name(), Some("Done"));
    assert_eq!(PROGRESS_FIELDS[0].visibility(), FieldVisibility::VariantInherited);
    assert_eq!(DONE_FIELDS[0].visibility(), FieldVisibility::VariantInherited);
}
