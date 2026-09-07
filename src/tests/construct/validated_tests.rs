// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Tests validated construction input inspection in both erased-value modes.

use std::any::TypeId;

use super::error_tests::FIXTURE_FIELDS;
use crate::construct::ConstructionError;
use crate::construct::ConstructionUnavailableReason;
use crate::construct::NamedConstructionInput;
use crate::construct::StructUpdateInput;
use crate::construct::TupleConstructionInput;
use crate::construct::UpdateField;
use crate::construct::UpdateFieldPolicy;
use crate::construct::validate_unit;
use crate::construct::validate_update;
use crate::value::DynamicOwned;
use crate::value::Local;
use crate::value::ThreadSafe;

static UPDATE_FIELDS: [UpdateField; 1] = [UpdateField::allowed(&FIXTURE_FIELDS[0])];

/// Reads the exact local erased-value type identity.
fn local_type_id(value: &DynamicOwned<Local>) -> TypeId {
    value.as_any().expect("local owned values use Any storage").type_id()
}

/// Reads the exact thread-safe erased-value type identity.
fn thread_safe_type_id(value: &DynamicOwned<ThreadSafe>) -> TypeId {
    value
        .as_any()
        .expect("thread-safe owned values use Any storage")
        .type_id()
}

/// Verifies empty unit validation exposes and consumes its descriptor-ordered
/// values in both supported modes.
#[test]
fn test_validated_unit_input_inspection_supports_both_modes() {
    let local = validate_unit::<Local>(&[]).expect("an empty unit contract must validate");
    assert!(local.values().is_empty());
    assert!(format!("{local:?}").contains("value_count: 0"));
    assert!(local.into_values().is_empty());

    let thread_safe = validate_unit::<ThreadSafe>(&[]).expect("an empty unit contract must validate");
    assert!(thread_safe.values().is_empty());
    assert!(format!("{thread_safe:?}").contains("value_count: 0"));
    assert!(thread_safe.into_values().is_empty());
}

/// Verifies successful empty updates expose and consume their exact base and
/// ordered override collection in both supported modes.
#[test]
fn test_validated_update_input_inspection_supports_both_modes() {
    let local_input = StructUpdateInput::new(
        DynamicOwned::<Local>::new(7_u8),
        NamedConstructionInput::<Local>::new(Vec::<(&str, DynamicOwned<Local>)>::new()),
    );
    assert_eq!(local_type_id(local_input.base()), TypeId::of::<u8>());
    assert!(local_input.overrides().fields().is_empty());
    assert!(format!("{local_input:?}").contains("override_count: 0"));
    let local = validate_update(local_input, TypeId::of::<u8>(), &[], local_type_id)
        .expect("an exact local base with no overrides must validate");
    assert_eq!(local_type_id(local.base()), TypeId::of::<u8>());
    assert!(local.overrides().is_empty());
    assert!(format!("{local:?}").contains("override_count: 0"));
    let (base, overrides) = local.into_parts();
    assert_eq!(
        base.downcast::<u8>()
            .unwrap_or_else(|_| panic!("the exact base type is retained")),
        7
    );
    assert!(overrides.is_empty());

    let thread_safe_input = StructUpdateInput::new(
        DynamicOwned::<ThreadSafe>::new(11_u8),
        NamedConstructionInput::<ThreadSafe>::new(Vec::<(&str, DynamicOwned<ThreadSafe>)>::new()),
    );
    assert_eq!(thread_safe_type_id(thread_safe_input.base()), TypeId::of::<u8>());
    assert!(thread_safe_input.overrides().fields().is_empty());
    assert!(format!("{thread_safe_input:?}").contains("override_count: 0"));
    let thread_safe = validate_update(thread_safe_input, TypeId::of::<u8>(), &[], thread_safe_type_id)
        .expect("an exact thread-safe base with no overrides must validate");
    assert_eq!(thread_safe_type_id(thread_safe.base()), TypeId::of::<u8>());
    assert!(thread_safe.overrides().is_empty());
    assert!(format!("{thread_safe:?}").contains("override_count: 0"));
    let (base, overrides) = thread_safe.into_parts();
    assert_eq!(
        base.downcast::<u8>()
            .unwrap_or_else(|_| panic!("the exact base type is retained")),
        11
    );
    assert!(overrides.is_empty());

    let failure = validate_update(
        StructUpdateInput::new(
            DynamicOwned::<ThreadSafe>::new(12_u16),
            NamedConstructionInput::<ThreadSafe>::new(Vec::<(&str, DynamicOwned<ThreadSafe>)>::new()),
        ),
        TypeId::of::<u8>(),
        &[],
        thread_safe_type_id,
    )
    .expect_err("a mismatched thread-safe base must preserve its input");
    assert!(matches!(
        failure.error(),
        ConstructionError::BaseTypeMismatch { mismatch }
            if mismatch.expected() == TypeId::of::<u8>()
                && mismatch.actual() == TypeId::of::<u16>()
    ));
    assert_eq!(failure.values().len(), 1);
}

/// Verifies thread-safe caller input containers retain order and expose their
/// non-value diagnostic summaries before validation consumes them.
#[test]
fn test_thread_safe_construction_input_inspection_preserves_order() {
    let named = NamedConstructionInput::new([
        ("left", DynamicOwned::<ThreadSafe>::new(1_u8)),
        ("right", DynamicOwned::<ThreadSafe>::new(2_u8)),
    ]);
    assert_eq!(named.fields()[0].0.as_ref(), "left");
    assert!(format!("{named:?}").contains("left"));
    let fields = named.into_fields();
    assert_eq!(fields.len(), 2);

    let tuple = TupleConstructionInput::new([
        DynamicOwned::<ThreadSafe>::new(3_u8),
        DynamicOwned::<ThreadSafe>::new(4_u8),
    ]);
    assert_eq!(tuple.values().len(), 2);
    assert!(format!("{tuple:?}").contains("value_count: 2"));
    assert_eq!(tuple.into_values().len(), 2);
}

/// Verifies update-field policies and successful typed overrides expose their
/// descriptor index and value in both erased modes.
#[test]
fn test_validated_override_inspection_supports_both_modes() {
    let allowed = UpdateField::allowed(&FIXTURE_FIELDS[0]);
    assert_eq!(allowed.descriptor().index(), 0);
    assert_eq!(allowed.policy(), UpdateFieldPolicy::Allowed);
    assert!(format!("{allowed:?}").contains("UpdateField"));
    let unavailable = UpdateField::unavailable(&FIXTURE_FIELDS[0], ConstructionUnavailableReason::UpdateForbidden);
    assert!(matches!(unavailable.policy(), UpdateFieldPolicy::Unavailable(_)));

    let local = validate_update(
        StructUpdateInput::new(
            DynamicOwned::<Local>::new(7_u32),
            NamedConstructionInput::new([("renamed", DynamicOwned::<Local>::new(8_u8))]),
        ),
        TypeId::of::<u32>(),
        &UPDATE_FIELDS,
        local_type_id,
    )
    .expect("an exact local override must validate");
    assert_eq!(local.overrides()[0].index(), 0);
    assert_eq!(local.overrides()[0].value().downcast_ref::<u8>(), Some(&8));
    assert!(format!("{:?}", local.overrides()[0]).contains("ValidatedOverride"));
    let (_, overrides) = local.into_parts();
    let (index, value) = overrides.into_vec().remove(0).into_parts();
    assert_eq!(index, 0);
    assert_eq!(value.downcast_ref::<u8>(), Some(&8));

    let thread_safe = validate_update(
        StructUpdateInput::new(
            DynamicOwned::<ThreadSafe>::new(9_u32),
            NamedConstructionInput::new([("renamed", DynamicOwned::<ThreadSafe>::new(10_u8))]),
        ),
        TypeId::of::<u32>(),
        &UPDATE_FIELDS,
        thread_safe_type_id,
    )
    .expect("an exact thread-safe override must validate");
    assert_eq!(thread_safe.overrides()[0].index(), 0);
    assert_eq!(thread_safe.overrides()[0].value().downcast_ref::<u8>(), Some(&10));
    assert!(format!("{:?}", thread_safe.overrides()[0]).contains("ValidatedOverride"));
    let (_, overrides) = thread_safe.into_parts();
    let (index, value) = overrides.into_vec().remove(0).into_parts();
    assert_eq!(index, 0);
    assert_eq!(value.downcast_ref::<u8>(), Some(&10));
}
