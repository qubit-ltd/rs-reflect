// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![allow(unsafe_code)]
use super::ConstructField;
use super::ConstructionAvailability;
use super::ConstructionError;
use super::ConstructionFieldPolicy;
use super::ConstructionFields;
use super::ConstructionRecovery;
use super::ConstructionShape;
use super::ConstructionTarget;
use super::ConstructionUnavailableReason;
use super::NamedFields;
use super::UnitFields;
use super::UpdateError;
use super::UpdateFieldPolicy;
use super::UpdateInput;
use super::UpdateUnavailableReason;
use super::construction_error::public_construction_error;
use super::recovered_construction_input::recovered_input;
use super::update_error::public_update_error;
use crate::runtime::construct as runtime;
use crate::runtime::construct::ConstructionError as RuntimeConstructionError;
use crate::runtime::construct::RecoveredConstructionInput as RuntimeRecoveredInput;
use crate::runtime::construct::UpdateError as RuntimeUpdateError;
use crate::runtime::descriptor::field::Visibility;
use crate::runtime::identity::TypeIdentity;
use crate::runtime::value::TypeMismatch;
use crate::value::Local;
use crate::value::ThreadSafe;

#[crate::Reflect]
#[derive(Debug, PartialEq)]
pub struct PublicRecord {
    pub id: u32,
    pub name: String,
}

#[crate::Reflect]
#[derive(Debug, PartialEq)]
pub enum PublicEvent {
    Named { id: u32 },
    Unit,
}

fn mismatch() -> TypeMismatch {
    TypeMismatch::new(
        TypeIdentity {
            type_id: std::any::TypeId::of::<u32>(),
            rust_name: "u32",
        },
        TypeIdentity {
            type_id: std::any::TypeId::of::<String>(),
            rust_name: "String",
        },
    )
}

fn target() -> TypeIdentity {
    TypeIdentity {
        type_id: std::any::TypeId::of::<u32>(),
        rust_name: "u32",
    }
}

#[test]
fn test_construction_errors_preserve_every_runtime_error_detail() {
    let mismatch = mismatch();
    let unavailable = ConstructionAvailability::Unavailable {
        reasons: Box::new([ConstructionUnavailableReason::PrivateRequiredField]),
    };
    let cases = [
        (
            RuntimeConstructionError::MissingProvider,
            ConstructionError::MissingProvider,
        ),
        (
            RuntimeConstructionError::AmbiguousProvider {
                target: target(),
                count: 2,
            },
            ConstructionError::AmbiguousProvider {
                target: "u32",
                count: 2,
            },
        ),
        (
            RuntimeConstructionError::WrongTargetKind,
            ConstructionError::WrongTargetKind,
        ),
        (
            RuntimeConstructionError::Unavailable(unavailable.clone()),
            ConstructionError::Unavailable(unavailable),
        ),
        (
            RuntimeConstructionError::TargetTypeMismatch(mismatch.clone()),
            ConstructionError::TargetTypeMismatch(mismatch.clone()),
        ),
        (
            RuntimeConstructionError::ShapeMismatch {
                expected: ConstructionShape::Named,
                actual: ConstructionShape::Positional,
            },
            ConstructionError::ShapeMismatch {
                expected: ConstructionShape::Named,
                actual: ConstructionShape::Positional,
            },
        ),
        (
            RuntimeConstructionError::MissingField { field: "name" },
            ConstructionError::MissingField { field: "name" },
        ),
        (
            RuntimeConstructionError::DuplicateField { field: "name" },
            ConstructionError::DuplicateField { field: "name" },
        ),
        (
            RuntimeConstructionError::UnknownField { field: "extra".into() },
            ConstructionError::UnknownField { field: "extra".into() },
        ),
        (
            RuntimeConstructionError::ProviderOnlyField { field: "id" },
            ConstructionError::ProviderOnlyField { field: "id" },
        ),
        (
            RuntimeConstructionError::FieldUnavailable {
                field: "id",
                reason: ConstructionUnavailableReason::FieldNotReflectable,
            },
            ConstructionError::FieldUnavailable {
                field: "id",
                reason: ConstructionUnavailableReason::FieldNotReflectable,
            },
        ),
        (
            RuntimeConstructionError::FieldNotPublic { field: "id" },
            ConstructionError::FieldNotPublic { field: "id" },
        ),
        (
            RuntimeConstructionError::ValueTypeMismatch {
                field: "id",
                mismatch: mismatch.clone(),
            },
            ConstructionError::ValueTypeMismatch {
                field: "id",
                mismatch: mismatch.clone(),
            },
        ),
        (
            RuntimeConstructionError::TooManyValues {
                expected_max: 2,
                actual: 3,
            },
            ConstructionError::TooManyValues {
                expected_max: 2,
                actual: 3,
            },
        ),
    ];

    for (runtime, expected) in cases {
        assert_eq!(public_construction_error(&runtime), expected);
    }
}

#[test]
fn test_update_errors_preserve_every_runtime_error_detail() {
    let mismatch = mismatch();
    let unavailable = ConstructionAvailability::Unavailable {
        reasons: Box::new([ConstructionUnavailableReason::UnsupportedShape]),
    };
    let cases = [
        (
            RuntimeUpdateError::BaseTypeMismatch(mismatch.clone()),
            UpdateError::BaseTypeMismatch(mismatch.clone()),
        ),
        (
            RuntimeUpdateError::TargetTypeMismatch(mismatch.clone()),
            UpdateError::TargetTypeMismatch(mismatch.clone()),
        ),
        (RuntimeUpdateError::WrongTargetKind, UpdateError::WrongTargetKind),
        (RuntimeUpdateError::MissingProvider, UpdateError::MissingProvider),
        (
            RuntimeUpdateError::AmbiguousProvider {
                target: target(),
                count: 3,
            },
            UpdateError::AmbiguousProvider {
                target: "u32",
                count: 3,
            },
        ),
        (
            RuntimeUpdateError::Unavailable(unavailable.clone()),
            UpdateError::Unavailable(unavailable),
        ),
        (
            RuntimeUpdateError::ShapeMismatch {
                expected: ConstructionShape::Named,
                actual: ConstructionShape::Unit,
            },
            UpdateError::ShapeMismatch {
                expected: ConstructionShape::Named,
                actual: ConstructionShape::Unit,
            },
        ),
        (
            RuntimeUpdateError::MissingField { field: "name" },
            UpdateError::MissingField { field: "name" },
        ),
        (
            RuntimeUpdateError::DuplicateField { field: "name" },
            UpdateError::DuplicateField { field: "name" },
        ),
        (
            RuntimeUpdateError::UnknownField { field: "extra".into() },
            UpdateError::UnknownField { field: "extra".into() },
        ),
        (
            RuntimeUpdateError::ProviderOnlyField { field: "id" },
            UpdateError::ProviderOnlyField { field: "id" },
        ),
        (
            RuntimeUpdateError::FieldUnavailable {
                field: "id",
                reason: ConstructionUnavailableReason::NoProvider,
            },
            UpdateError::FieldUnavailable {
                field: "id",
                reason: ConstructionUnavailableReason::NoProvider,
            },
        ),
        (
            RuntimeUpdateError::FieldNotPublic { field: "id" },
            UpdateError::FieldNotPublic { field: "id" },
        ),
        (
            RuntimeUpdateError::ValueTypeMismatch {
                field: "id",
                mismatch: mismatch.clone(),
            },
            UpdateError::ValueTypeMismatch {
                field: "id",
                mismatch: mismatch.clone(),
            },
        ),
        (
            RuntimeUpdateError::TooManyValues {
                expected_max: 1,
                actual: 2,
            },
            UpdateError::TooManyValues {
                expected_max: 1,
                actual: 2,
            },
        ),
        (
            RuntimeUpdateError::FieldPreserved { field: "id" },
            UpdateError::FieldPreserved { field: "id" },
        ),
        (
            RuntimeUpdateError::FieldUpdateUnavailable {
                field: "id",
                reason: UpdateUnavailableReason::FieldNotReplaceable,
            },
            UpdateError::FieldUpdateUnavailable {
                field: "id",
                reason: UpdateUnavailableReason::FieldNotReplaceable,
            },
        ),
        (
            RuntimeUpdateError::UpdateUnavailable(UpdateUnavailableReason::NoProvider),
            UpdateError::UpdateUnavailable(UpdateUnavailableReason::NoProvider),
        ),
    ];

    for (runtime, expected) in cases {
        assert_eq!(public_update_error(&runtime), expected);
    }
}

#[test]
fn test_recovered_inputs_and_field_views_preserve_shape_and_bounds() {
    let named = recovered_input(RuntimeRecoveredInput::Named(runtime::NamedFields::<Local>::new()));
    let positional = recovered_input(RuntimeRecoveredInput::Positional(
        runtime::PositionalFields::<Local>::new(),
    ));
    let unit = recovered_input(RuntimeRecoveredInput::<Local>::Unit(runtime::UnitFields::new()));
    assert_eq!(ConstructionRecovery { input: &named }.shape(), ConstructionShape::Named);
    assert_eq!(
        ConstructionRecovery { input: &positional }.shape(),
        ConstructionShape::Positional
    );
    assert_eq!(ConstructionRecovery { input: &unit }.shape(), ConstructionShape::Unit);

    let fields = [ConstructField::typed::<u32>(
        "id",
        Visibility::Public,
        ConstructionFieldPolicy::Required,
        UpdateFieldPolicy::Replaceable,
    )];
    let view = ConstructionFields::new(&fields);
    assert_eq!(view.len(), 1);
    assert!(!view.is_empty());
    assert_eq!(view.get(0).map(ConstructField::name), Some("id"));
    assert!(view.get(1).is_none());
    assert!(view.get(usize::MAX).is_none());
    assert_eq!(view.iter().map(ConstructField::name).collect::<Vec<_>>(), ["id"]);

    let empty = ConstructionFields::new(&[]);
    assert_eq!(empty.len(), 0);
    assert!(empty.is_empty());
    assert!(empty.get(0).is_none());
    assert_eq!(empty.iter().count(), 0);
}

#[test]
fn test_public_facade_constructs_and_updates_local_and_thread_safe_values() {
    let descriptor = crate::TypeDescriptor::of::<PublicRecord>();
    let plan = descriptor.construction();
    assert!(matches!(plan.target(), ConstructionTarget::Struct(_)));
    assert_eq!(plan.shape(), ConstructionShape::Named);
    assert_eq!(
        plan.fields().iter().map(ConstructField::name).collect::<Vec<_>>(),
        ["id", "name"]
    );

    let local: PublicRecord = descriptor
        .construct(
            NamedFields::<Local>::new()
                .field("id", 1_u32)
                .field("name", "local".to_owned()),
        )
        .expect("named fields construct a local typed value");
    assert_eq!(
        local,
        PublicRecord {
            id: 1,
            name: "local".to_owned()
        }
    );

    let thread_safe: PublicRecord = descriptor
        .construct(
            NamedFields::<ThreadSafe>::new()
                .field("id", 2_u32)
                .field("name", "safe".to_owned()),
        )
        .expect("named fields construct a thread-safe typed value");
    assert_eq!(
        thread_safe,
        PublicRecord {
            id: 2,
            name: "safe".to_owned()
        }
    );

    let dynamic = descriptor
        .construct_dynamic(
            NamedFields::<ThreadSafe>::new()
                .field("id", 3_u32)
                .field("name", "dynamic".to_owned()),
        )
        .expect("named fields construct a thread-safe dynamic value");
    assert_eq!(
        dynamic.downcast_ref::<PublicRecord>(),
        Some(&PublicRecord {
            id: 3,
            name: "dynamic".to_owned()
        })
    );

    let unchecked: PublicRecord = unsafe {
        descriptor.construct_unchecked(
            NamedFields::<ThreadSafe>::new()
                .field("id", 4_u32)
                .field("name", "unchecked".to_owned()),
        )
    }
    .expect("unchecked typed construction preserves valid field values");
    assert_eq!(unchecked.id, 4);

    let dynamic_unchecked = unsafe {
        descriptor.construct_dynamic_unchecked(
            NamedFields::<ThreadSafe>::new()
                .field("id", 5_u32)
                .field("name", "dynamic unchecked".to_owned()),
        )
    }
    .expect("unchecked dynamic construction preserves valid field values");
    assert_eq!(
        dynamic_unchecked.downcast_ref::<PublicRecord>().map(|value| value.id),
        Some(5)
    );

    let updated: PublicRecord = descriptor
        .update::<_, ThreadSafe>(UpdateInput::<ThreadSafe>::new(thread_safe).field("name", "updated".to_owned()))
        .expect("thread-safe typed updates replace selected fields");
    assert_eq!(
        updated,
        PublicRecord {
            id: 2,
            name: "updated".to_owned()
        }
    );

    let unchecked_typed_update: PublicRecord = unsafe {
        plan.update_unchecked::<_, ThreadSafe>(
            UpdateInput::<ThreadSafe>::new(PublicRecord {
                id: 9,
                name: "before".to_owned(),
            })
            .field("name", "typed unchecked".to_owned()),
        )
    }
    .expect("thread-safe unchecked typed update preserves valid replacements");
    assert_eq!(unchecked_typed_update.name, "typed unchecked");

    let local_dynamic_update = descriptor
        .update_dynamic::<Local>(
            UpdateInput::<Local>::new(PublicRecord {
                id: 10,
                name: "before".to_owned(),
            })
            .field("name", "local dynamic".to_owned()),
        )
        .expect("local dynamic update replaces selected fields");
    assert_eq!(
        local_dynamic_update
            .downcast_ref::<PublicRecord>()
            .map(|value| value.name.as_str()),
        Some("local dynamic")
    );

    let dynamic_updated = descriptor
        .update_dynamic::<ThreadSafe>(UpdateInput::from_dynamic(dynamic).field("id", 6_u32))
        .expect("dynamic updates accept a dynamic base value");
    assert_eq!(
        dynamic_updated.downcast_ref::<PublicRecord>().map(|value| value.id),
        Some(6)
    );

    let dynamic_unchecked_update = unsafe {
        descriptor.update_dynamic_unchecked::<ThreadSafe>(
            UpdateInput::<ThreadSafe>::new(PublicRecord {
                id: 7,
                name: "before".to_owned(),
            })
            .field("name", "after".to_owned()),
        )
    }
    .expect("unchecked dynamic update handles a valid replacement");
    assert_eq!(
        dynamic_unchecked_update
            .downcast_ref::<PublicRecord>()
            .map(|value| value.name.as_str()),
        Some("after")
    );

    let local_unchecked_update = unsafe {
        descriptor.update_dynamic_unchecked::<Local>(
            UpdateInput::<Local>::new(PublicRecord {
                id: 8,
                name: "before".to_owned(),
            })
            .field("name", "local after".to_owned()),
        )
    }
    .expect("local unchecked dynamic update handles a valid replacement");
    assert_eq!(
        local_unchecked_update
            .downcast_ref::<PublicRecord>()
            .map(|value| value.name.as_str()),
        Some("local after")
    );
}

#[test]
fn test_enum_variant_facade_covers_named_and_unit_construction_paths() {
    let descriptor = crate::TypeDescriptor::of::<PublicEvent>();
    let named = descriptor.variant("Named").expect("named variant is reflected");
    assert!(matches!(named.construction().target(), ConstructionTarget::Variant(_)));
    let event: PublicEvent = named
        .construct(NamedFields::<Local>::new().field("id", 11_u32))
        .expect("named enum variant is constructible by field name");
    assert_eq!(event, PublicEvent::Named { id: 11 });

    let dynamic = named
        .construct_dynamic(NamedFields::<ThreadSafe>::new().field("id", 12_u32))
        .expect("named enum variant supports dynamic construction");
    assert_eq!(
        dynamic.downcast_ref::<PublicEvent>(),
        Some(&PublicEvent::Named { id: 12 })
    );

    let unit = descriptor.variant("Unit").expect("unit variant is reflected");
    let event: PublicEvent = unit
        .construct(UnitFields::new())
        .expect("unit enum variant is constructed with unit input");
    assert_eq!(event, PublicEvent::Unit);

    let dynamic_unchecked = unsafe { unit.construct_dynamic_unchecked(UnitFields::new()) }
        .expect("unit enum variant supports unchecked dynamic construction");
    assert_eq!(
        dynamic_unchecked.downcast_ref::<PublicEvent>(),
        Some(&PublicEvent::Unit)
    );
}

#[test]
fn test_public_failures_format_and_retain_update_and_construction_inputs() {
    let descriptor = crate::TypeDescriptor::of::<PublicRecord>();
    let failure = descriptor
        .construct::<PublicRecord, _>(NamedFields::<Local>::new().field("id", "wrong".to_owned()))
        .expect_err("wrong field types are reported");
    assert!(matches!(
        failure.error(),
        ConstructionError::ValueTypeMismatch { field: "id", .. }
    ));
    assert!(format!("{failure:?}").contains("shape: Named"));
    assert_eq!(failure.recovery().shape(), ConstructionShape::Named);

    let update_failure = descriptor
        .update::<PublicRecord, Local>(
            UpdateInput::<Local>::new(PublicRecord {
                id: 1,
                name: "base".to_owned(),
            })
            .field("id", "wrong".to_owned()),
        )
        .expect_err("wrong update field types are reported");
    assert!(matches!(
        update_failure.error(),
        UpdateError::ValueTypeMismatch { field: "id", .. }
    ));
    assert!(format!("{update_failure:?}").contains("UpdateFailure"));
    assert_eq!(
        update_failure.base().downcast_ref::<PublicRecord>(),
        Some(&PublicRecord {
            id: 1,
            name: "base".to_owned()
        })
    );
    assert_eq!(
        update_failure.recovery().base().downcast_ref::<PublicRecord>(),
        Some(&PublicRecord {
            id: 1,
            name: "base".to_owned()
        })
    );
}
