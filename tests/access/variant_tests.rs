// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::TypeId;

use qubit_reflect::Reflect;
use qubit_reflect::ReflectionContext;
use qubit_reflect::descriptor::FieldAccessError;
use qubit_reflect::descriptor::FieldSetError;
use qubit_reflect::descriptor::TypeDescriptor;

#[Reflect]
pub enum Event {
    Progress { value: u32 },
    Done { value: u32 },
}

struct NotAnEvent;

#[test]
fn variant_active_check_and_public_payload_access_are_typed() {
    let descriptor = TypeDescriptor::of_in::<Event>(ReflectionContext::global());
    let progress = descriptor.variant("Progress").expect("variant is described");
    let done = descriptor.variant("Done").expect("variant is described");
    let event = Event::Progress { value: 5 };

    assert!(progress.is_active(&event).expect("enum target type matches"));
    assert!(!done.is_active(&event).expect("enum target type matches"));
    assert!(
        done.is_active(&Event::Done { value: 9 })
            .expect("enum target type matches")
    );
    let value: &u32 = progress.field("value").unwrap().get(&event).unwrap();
    assert_eq!(*value, 5);
}

#[test]
fn inactive_variant_field_set_recovers_the_replacement() {
    let descriptor = TypeDescriptor::of_in::<Event>(ReflectionContext::global());
    let field = descriptor
        .variant("Progress")
        .and_then(|variant| variant.field("value"))
        .expect("payload field is described");
    let mut event = Event::Done { value: 9 };

    let failure = field
        .set(&mut event, 42_u32)
        .expect_err("inactive variant payload cannot be mutated");
    assert!(matches!(failure.error(), FieldSetError::InactiveVariant { .. }));
    let (error, replacement) = failure.into_parts();
    assert!(matches!(error, FieldSetError::InactiveVariant { .. }));
    assert_eq!(replacement, 42_u32);
    assert!(matches!(event, Event::Done { value: 9 }));
}

#[test]
fn variant_field_errors_keep_root_type_and_variant_identity() {
    let descriptor = TypeDescriptor::of_in::<Event>(ReflectionContext::global());
    let progress = descriptor
        .variant("Progress")
        .and_then(|variant| variant.field("value"))
        .expect("progress payload is described");
    let done = descriptor
        .variant("Done")
        .and_then(|variant| variant.field("value"))
        .expect("done payload is described");

    let progress_error = progress
        .get::<_, u32>(&NotAnEvent)
        .expect_err("a non-enum target is rejected");
    let done_error = done
        .get::<_, u32>(&NotAnEvent)
        .expect_err("a non-enum target is rejected");
    let FieldAccessError::TargetTypeMismatch(mismatch) = progress_error else {
        panic!("the progress error must classify the target mismatch");
    };
    let FieldAccessError::TargetTypeMismatch(_) = done_error else {
        panic!("the done error must classify the target mismatch");
    };
    assert_eq!(progress.index(), 0);
    assert_eq!(done.index(), 0);
    assert_eq!(progress.declaring_variant().unwrap().index(), 0);
    assert_eq!(done.declaring_variant().unwrap().index(), 1);
    assert_eq!(progress.name(), "value");
    assert_eq!(done.name(), "value");
    assert_eq!(progress.declaring_variant().unwrap().name(), "Progress");
    assert_eq!(done.declaring_variant().unwrap().name(), "Done");
    assert_eq!(mismatch.expected(), TypeId::of::<Event>());
    assert_eq!(mismatch.actual(), TypeId::of::<NotAnEvent>());
}
