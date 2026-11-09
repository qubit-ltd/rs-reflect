// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![cfg(feature = "derive")]

use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use qubit_reflect::Reflect;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::construct::UpdateError;
use qubit_reflect::construct::UpdateInput;
use qubit_reflect::value::Local;
use qubit_reflect::value::ThreadSafe;

#[Reflect]
#[derive(Debug, PartialEq)]
struct Pair(pub u32, pub String);

#[Reflect]
#[derive(Debug, PartialEq)]
struct PrivatePair(pub u32, String);

static DROPPED_REPLACEMENTS: AtomicUsize = AtomicUsize::new(0);

#[Reflect]
struct DroppedReplacement;

impl Drop for DroppedReplacement {
    fn drop(&mut self) {
        DROPPED_REPLACEMENTS.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn test_typed_local_named_replacement_preserves_omitted_tuple_field() {
    let updated: Pair = TypeDescriptor::of::<Pair>()
        .update(UpdateInput::<Local>::new(Pair(1, "before".to_owned())).field("1", "after".to_owned()))
        .expect("named replacement must work for tuple fields");
    assert_eq!(updated, Pair(1, "after".to_owned()));
}

#[test]
fn test_dynamic_local_and_thread_safe_named_replacements_work() {
    let local = TypeDescriptor::of::<Pair>()
        .update_dynamic(UpdateInput::<Local>::new(Pair(2, "before".to_owned())).field("0", 3_u32))
        .expect("local dynamic tuple update");
    assert_eq!(local.downcast::<Pair>().unwrap(), Pair(3, "before".to_owned()));

    let shared = TypeDescriptor::of::<Pair>()
        .update_dynamic(UpdateInput::<ThreadSafe>::new(Pair(4, "before".to_owned())).field("1", "after".to_owned()))
        .expect("thread safe dynamic tuple update");
    assert_eq!(shared.downcast::<Pair>().unwrap(), Pair(4, "after".to_owned()));
}

#[test]
fn test_empty_tuple_update_is_valid_and_unknown_or_duplicate_fields_are_rejected() {
    let unchanged: Pair = TypeDescriptor::of::<Pair>()
        .update(UpdateInput::<Local>::new(Pair(5, "same".to_owned())))
        .expect("empty tuple update preserves every field");
    assert_eq!(unchanged, Pair(5, "same".to_owned()));

    let unknown = TypeDescriptor::of::<Pair>()
        .update::<Pair, _>(UpdateInput::<Local>::new(Pair(6, "base".to_owned())).field("missing", 1_u32))
        .expect_err("unknown field is rejected");
    assert!(matches!(unknown.error(), UpdateError::UnknownField { field } if field.as_ref() == "missing"));

    let duplicate = TypeDescriptor::of::<Pair>()
        .update::<Pair, _>(
            UpdateInput::<Local>::new(Pair(7, "base".to_owned()))
                .field("0", 8_u32)
                .field("0", 9_u32),
        )
        .expect_err("duplicate field is rejected");
    assert!(matches!(duplicate.error(), UpdateError::DuplicateField { field: "0" }));
}

#[test]
fn test_failed_tuple_updates_recover_base_and_replacement_values() {
    DROPPED_REPLACEMENTS.store(0, Ordering::SeqCst);
    let base = Pair(9, "unchanged".to_owned());
    let failure = TypeDescriptor::of::<Pair>()
        .update::<Pair, _>(UpdateInput::<Local>::new(base).field("1", DroppedReplacement))
        .expect_err("wrong replacement type");
    assert!(matches!(
        failure.error(),
        UpdateError::ValueTypeMismatch { field: "1", .. }
    ));
    assert_eq!(
        failure.base().downcast_ref::<Pair>(),
        Some(&Pair(9, "unchanged".to_owned()))
    );
    assert_eq!(
        DROPPED_REPLACEMENTS.load(Ordering::SeqCst),
        0,
        "failed validation retains the replacement"
    );
    let retry = TypeDescriptor::of::<Pair>()
        .update::<Pair, _>(failure.into_input())
        .expect_err("recovered replacement remains unchanged");
    assert!(matches!(
        retry.error(),
        UpdateError::ValueTypeMismatch { field: "1", .. }
    ));
    assert_eq!(
        DROPPED_REPLACEMENTS.load(Ordering::SeqCst),
        0,
        "retry failure still retains the replacement"
    );
    drop(retry);
    assert_eq!(
        DROPPED_REPLACEMENTS.load(Ordering::SeqCst),
        1,
        "recovered input drops the replacement exactly once"
    );
}

#[test]
fn test_safe_update_rejects_private_tuple_field() {
    let failure = TypeDescriptor::of::<PrivatePair>()
        .update::<PrivatePair, _>(
            UpdateInput::<Local>::new(PrivatePair(1, "before".to_owned())).field("1", "after".to_owned()),
        )
        .expect_err("private tuple fields are not safely replaceable");
    assert!(matches!(failure.error(), UpdateError::FieldNotPublic { field: "1" }));
}

#[test]
fn test_unsafe_update_can_replace_private_tuple_field() {
    let updated: PrivatePair = unsafe {
        TypeDescriptor::of::<PrivatePair>()
            .update_unchecked(
                UpdateInput::<Local>::new(PrivatePair(1, "before".to_owned())).field("1", "after".to_owned()),
            )
            .expect("unsafe update bypasses visibility")
    };
    assert_eq!(updated, PrivatePair(1, "after".to_owned()));
}

#[test]
fn test_unit_update_is_valid() {
    #[Reflect]
    #[derive(Debug, PartialEq)]
    struct Unit;
    let updated: Unit = TypeDescriptor::of::<Unit>()
        .update(UpdateInput::<Local>::new(Unit))
        .expect("unit update with no replacements");
    assert_eq!(updated, Unit);
}
