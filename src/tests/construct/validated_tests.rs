// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::TypeId;

use super::error_tests::FIXTURE_FIELDS;
use crate::__private::descriptor;
use crate::construct::ConstructionError;
use crate::construct::ConstructionUnavailableReason;
use crate::construct::NamedConstructionInput;
use crate::construct::StructUpdateInput;
use crate::construct::TupleConstructionInput;
use crate::construct::UpdateField;
use crate::construct::UpdateFieldPolicy;
use crate::construct::validate_unit;
use crate::construct::validate_update;
use crate::descriptor::FieldDescriptor;
use crate::descriptor::OpaqueTypeDescriptor;
use crate::descriptor::StructKind;
use crate::descriptor::TypeDescriptor;
use crate::descriptor::TypeRef;
use crate::identity::Visibility;
use crate::value::DynamicOwned;
use crate::value::Local;
use crate::value::ThreadSafe;

struct PublicUpdateFixture;

/// Returns the root descriptor used by public update validation.
fn public_update_descriptor() -> &'static TypeDescriptor {
    &PUBLIC_UPDATE_DESCRIPTOR
}

static UPDATE_U8_TYPE: OpaqueTypeDescriptor = descriptor::opaque_member::<u8>();
static UPDATE_U8_TYPE_REF: TypeRef = TypeRef::Opaque(&UPDATE_U8_TYPE);
static PUBLIC_UPDATE_DESCRIPTOR_FIELDS: [FieldDescriptor; 1] = [descriptor::field(
    public_update_descriptor,
    0,
    Some("value"),
    Some("value"),
    &UPDATE_U8_TYPE_REF,
    Visibility::Public,
)];
static PUBLIC_UPDATE_DESCRIPTOR: TypeDescriptor = descriptor::struct_type::<PublicUpdateFixture>(
    "PublicUpdateFixture",
    StructKind::Named,
    &PUBLIC_UPDATE_DESCRIPTOR_FIELDS,
);
static UPDATE_FIELDS: [UpdateField; 1] = [UpdateField::allowed(&PUBLIC_UPDATE_DESCRIPTOR_FIELDS[0])];

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

/// Verifies unit validation still supports both erased value modes.
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

    let value = DynamicOwned::<ThreadSafe>::new(11_u8);
    assert_eq!(thread_safe_type_id(&value), TypeId::of::<u8>());
}

/// Verifies typed local update inputs retain their base and override data, and
/// validation failures recover the caller-owned inputs in order.
#[test]
fn test_validated_update_input_inspection_and_recovery() {
    let local_input = StructUpdateInput::new(7_u8, NamedConstructionInput::new());
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

    let failure = validate_update(
        StructUpdateInput::new(12_u16, NamedConstructionInput::new().field("value", 9_u8)),
        TypeId::of::<u8>(),
        &UPDATE_FIELDS,
        local_type_id,
    )
    .expect_err("a mismatched base must preserve the base and override");
    assert!(matches!(
        failure.error(),
        ConstructionError::BaseTypeMismatch { mismatch }
            if mismatch.expected() == TypeId::of::<u8>()
                && mismatch.actual() == TypeId::of::<u16>()
    ));
    let (_, recovered) = failure.into_any_parts();
    assert_eq!(recovered.len(), 2);
    assert!(matches!(recovered[0], crate::construct::RecoveredAnyValue::Base(_)));
    assert!(matches!(
        &recovered[1],
        crate::construct::RecoveredAnyValue::Named { name, value }
            if name.as_ref() == "value" && value.downcast_ref::<u8>() == Some(&9)
    ));

    let failure = validate_update(
        StructUpdateInput::new(7_u32, NamedConstructionInput::new().field("value", 10_u16)),
        TypeId::of::<u32>(),
        &UPDATE_FIELDS,
        local_type_id,
    )
    .expect_err("an override with a non-exact field type must be recovered");
    assert!(matches!(
        failure.error(),
        ConstructionError::ValueTypeMismatch { field, mismatch }
            if field.query_name() == Some("value")
                && mismatch.expected() == TypeId::of::<u8>()
                && mismatch.actual() == TypeId::of::<u16>()
    ));
    let (_, recovered) = failure.into_any_parts();
    assert_eq!(recovered.len(), 2);
    assert!(matches!(
        &recovered[0],
        crate::construct::RecoveredAnyValue::Base(base)
            if base.downcast_ref::<u32>() == Some(&7)
    ));
    assert!(matches!(
        &recovered[1],
        crate::construct::RecoveredAnyValue::Named { name, value }
            if name.as_ref() == "value" && value.downcast_ref::<u16>() == Some(&10)
    ));
}

/// Verifies new caller input builders preserve names and tuple positions.
#[test]
fn test_construction_input_builders_preserve_caller_order() {
    let named = NamedConstructionInput::new().field("left", 1_u8).field("right", 2_u8);
    assert_eq!(named.fields()[0].0.as_ref(), "left");
    assert_eq!(named.fields()[1].0.as_ref(), "right");
    assert!(format!("{named:?}").contains("left"));
    assert_eq!(named.into_fields().len(), 2);

    let tuple = TupleConstructionInput::new().value(3_u8).value(4_u8);
    assert_eq!(tuple.values().len(), 2);
    assert!(format!("{tuple:?}").contains("value_count: 2"));
    assert_eq!(tuple.into_values().len(), 2);
}

/// Verifies update policy metadata and validated local overrides preserve the
/// target field index and exact owned value.
#[test]
fn test_validated_override_inspection_preserves_policy_and_value() {
    let allowed = UpdateField::allowed(&FIXTURE_FIELDS[0]);
    assert_eq!(allowed.descriptor().index(), 0);
    assert_eq!(allowed.policy(), UpdateFieldPolicy::Allowed);
    assert!(format!("{allowed:?}").contains("UpdateField"));
    let unavailable = UpdateField::unavailable(&FIXTURE_FIELDS[0], ConstructionUnavailableReason::UpdateForbidden);
    assert!(matches!(unavailable.policy(), UpdateFieldPolicy::Unavailable(_)));

    let local = validate_update(
        StructUpdateInput::new(7_u32, NamedConstructionInput::new().field("value", 8_u8)),
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
}
