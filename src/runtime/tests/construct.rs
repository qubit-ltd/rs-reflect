// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![allow(unsafe_code)]

use std::any::TypeId;
use std::sync::OnceLock;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use crate::runtime::construct::ConstructField;
use crate::runtime::construct::ConstructProvider;
use crate::runtime::construct::ConstructVisibility;
use crate::runtime::construct::ConstructionAvailability;
use crate::runtime::construct::ConstructionDescriptor;
use crate::runtime::construct::ConstructionError;
use crate::runtime::construct::ConstructionFieldPolicy;
use crate::runtime::construct::ConstructionPlan;
use crate::runtime::construct::ConstructionProviders;
use crate::runtime::construct::ConstructionShape;
use crate::runtime::construct::ConstructionTarget;
use crate::runtime::construct::ConstructionTargetKind;
use crate::runtime::construct::NamedFields;
use crate::runtime::construct::PositionalFields;
use crate::runtime::construct::RecoveredConstructionInput;
use crate::runtime::construct::UnitFields;
use crate::runtime::construct::UpdateError;
use crate::runtime::construct::UpdateFieldPolicy;
use crate::runtime::construct::UpdateInput;
use crate::runtime::construct::UpdateUnavailableReason;
use crate::runtime::construct::resolve_plan;
use crate::runtime::construct::resolve_variant_plan;
use crate::runtime::descriptor::Reflect;
use crate::runtime::descriptor::TypeRegistration;
use crate::runtime::error::RecoverableFailure;
use crate::runtime::identity::TypeIdentity;
use crate::runtime::value::DynamicOwned;
use crate::runtime::value::Local;
use crate::runtime::value::ThreadSafe;
use crate::runtime::value::ValueFor;

#[derive(Debug, PartialEq)]
struct Record {
    count: u32,
    title: String,
    secret: bool,
}

#[derive(Debug, PartialEq)]
enum Choice {
    // Construction plans below address variant index 1, so keep index 0 in the fixture.
    #[allow(dead_code)]
    Empty,
    Coordinates(i32, i32),
}

#[derive(Debug, PartialEq)]
struct Marker;

#[derive(Debug)]
struct Wrong;

static FREEZE_PROVIDER_CALLS: AtomicUsize = AtomicUsize::new(0);
struct FreezeProbeProvider;
impl ConstructProvider<Local> for FreezeProbeProvider {
    fn construct(&self, _: Vec<Option<DynamicOwned<Local>>>) -> DynamicOwned<Local> {
        FREEZE_PROVIDER_CALLS.fetch_add(1, Ordering::SeqCst);
        DynamicOwned::new(Record {
            count: 0,
            title: String::new(),
            secret: false,
        })
    }
    fn update(&self, base: DynamicOwned<Local>, _: Vec<Option<DynamicOwned<Local>>>) -> DynamicOwned<Local> {
        FREEZE_PROVIDER_CALLS.fetch_add(1, Ordering::SeqCst);
        base
    }
}
static FREEZE_PROVIDER: FreezeProbeProvider = FreezeProbeProvider;
static FREEZE_PLAN: OnceLock<ConstructionPlan> = OnceLock::new();

fn freeze_probe_plan() -> &'static ConstructionPlan {
    FREEZE_PLAN.get_or_init(|| {
        ConstructionPlan::for_type::<Record>(
            ConstructionShape::Unit,
            &[],
            ConstructionProviders::from_runtime_providers(Some(&FREEZE_PROVIDER), None),
            ConstructionAvailability::Available,
        )
    })
}

macro_rules! reflect_type {
    ($type:ty, $registration:ident) => {
        impl Reflect for $type {
            fn __reflect_registration() -> &'static TypeRegistration<Self> {
                static $registration: TypeRegistration<$type> = TypeRegistration::new();
                &$registration
            }
        }
    };
}

reflect_type!(Record, RECORD_REGISTRATION);
reflect_type!(Choice, CHOICE_REGISTRATION);
reflect_type!(Marker, MARKER_REGISTRATION);
reflect_type!(Wrong, WRONG_REGISTRATION);

struct RecordProvider;

impl ConstructProvider<Local> for RecordProvider {
    fn construct(&self, fields: Vec<Option<DynamicOwned<Local>>>) -> DynamicOwned<Local> {
        let mut fields = fields.into_iter();
        DynamicOwned::new(Record {
            count: value(fields.next().unwrap()),
            title: value(fields.next().unwrap()),
            secret: fields.next().unwrap().map(|field| value(Some(field))).unwrap_or(false),
        })
    }

    fn update(&self, base: DynamicOwned<Local>, replacements: Vec<Option<DynamicOwned<Local>>>) -> DynamicOwned<Local> {
        let mut record = base.downcast::<Record>().ok().unwrap();
        let mut replacements = replacements.into_iter();
        if let Some(value) = replacements.next().unwrap() {
            record.count = value.downcast::<u32>().ok().unwrap();
        }
        if let Some(value) = replacements.next().unwrap() {
            record.title = value.downcast::<String>().ok().unwrap();
        }
        if let Some(value) = replacements.next().unwrap() {
            record.secret = value.downcast::<bool>().ok().unwrap();
        }
        DynamicOwned::new(record)
    }
}

impl ConstructProvider<ThreadSafe> for RecordProvider {
    fn construct(&self, fields: Vec<Option<DynamicOwned<ThreadSafe>>>) -> DynamicOwned<ThreadSafe> {
        let mut fields = fields.into_iter();
        DynamicOwned::new(Record {
            count: value(fields.next().unwrap()),
            title: value(fields.next().unwrap()),
            secret: fields.next().unwrap().map(|field| value(Some(field))).unwrap_or(false),
        })
    }

    fn update(
        &self,
        base: DynamicOwned<ThreadSafe>,
        replacements: Vec<Option<DynamicOwned<ThreadSafe>>>,
    ) -> DynamicOwned<ThreadSafe> {
        let mut record = base.downcast::<Record>().ok().unwrap();
        let mut replacements = replacements.into_iter();
        if let Some(value) = replacements.next().unwrap() {
            record.count = value.downcast::<u32>().ok().unwrap();
        }
        if let Some(value) = replacements.next().unwrap() {
            record.title = value.downcast::<String>().ok().unwrap();
        }
        if let Some(value) = replacements.next().unwrap() {
            record.secret = value.downcast::<bool>().ok().unwrap();
        }
        DynamicOwned::new(record)
    }
}

struct PairProvider;

impl ConstructProvider<Local> for PairProvider {
    fn construct(&self, fields: Vec<Option<DynamicOwned<Local>>>) -> DynamicOwned<Local> {
        let mut fields = fields.into_iter();
        DynamicOwned::new(Choice::Coordinates(
            value(fields.next().unwrap()),
            value(fields.next().unwrap()),
        ))
    }

    fn update(&self, base: DynamicOwned<Local>, _: Vec<Option<DynamicOwned<Local>>>) -> DynamicOwned<Local> {
        base
    }
}

struct MarkerProvider;

impl ConstructProvider<Local> for MarkerProvider {
    fn construct(&self, _: Vec<Option<DynamicOwned<Local>>>) -> DynamicOwned<Local> {
        DynamicOwned::new(Marker)
    }

    fn update(&self, base: DynamicOwned<Local>, _: Vec<Option<DynamicOwned<Local>>>) -> DynamicOwned<Local> {
        base
    }
}

fn value<M, T>(value: Option<DynamicOwned<M>>) -> T
where
    M: crate::runtime::value::ValueMode,
    T: ValueFor<M>,
{
    value.unwrap().downcast::<T>().ok().unwrap()
}

fn type_identity<T: 'static>() -> TypeIdentity {
    TypeIdentity {
        type_id: TypeId::of::<T>(),
        rust_name: std::any::type_name::<T>(),
    }
}

fn record_plan() -> ConstructionPlan {
    record_plan_with_visibility(ConstructVisibility::Public)
}

fn record_plan_with_visibility(secret_visibility: ConstructVisibility) -> ConstructionPlan {
    record_plan_with_policy(secret_visibility, ConstructionFieldPolicy::Defaulted)
}

fn record_plan_with_policy(
    secret_visibility: ConstructVisibility,
    secret_policy: ConstructionFieldPolicy,
) -> ConstructionPlan {
    let fields = Box::leak(Box::new([
        ConstructField {
            name: "count",
            value: type_identity::<u32>,
            visibility: ConstructVisibility::Public,
            policy: ConstructionFieldPolicy::Required,
            update_policy: UpdateFieldPolicy::Replaceable,
        },
        ConstructField {
            name: "title",
            value: type_identity::<String>,
            visibility: ConstructVisibility::Public,
            policy: ConstructionFieldPolicy::Required,
            update_policy: UpdateFieldPolicy::Replaceable,
        },
        ConstructField {
            name: "secret",
            value: type_identity::<bool>,
            visibility: secret_visibility,
            policy: secret_policy,
            update_policy: UpdateFieldPolicy::Preserved,
        },
    ]));
    let provider = Box::leak(Box::new(RecordProvider));
    ConstructionPlan::for_type::<Record>(
        ConstructionShape::Named,
        fields,
        ConstructionProviders::from_runtime_providers(Some(provider), Some(provider)),
        ConstructionAvailability::Available,
    )
}

fn record_plan_with_unavailable_update() -> ConstructionPlan {
    let fields = Box::leak(Box::new([
        ConstructField {
            name: "count",
            value: type_identity::<u32>,
            visibility: ConstructVisibility::Public,
            policy: ConstructionFieldPolicy::Required,
            update_policy: UpdateFieldPolicy::Unavailable(UpdateUnavailableReason::NoProvider),
        },
        ConstructField {
            name: "title",
            value: type_identity::<String>,
            visibility: ConstructVisibility::Public,
            policy: ConstructionFieldPolicy::Required,
            update_policy: UpdateFieldPolicy::Replaceable,
        },
        ConstructField {
            name: "secret",
            value: type_identity::<bool>,
            visibility: ConstructVisibility::Public,
            policy: ConstructionFieldPolicy::Defaulted,
            update_policy: UpdateFieldPolicy::Preserved,
        },
    ]));
    let provider = Box::leak(Box::new(RecordProvider));
    ConstructionPlan::for_type::<Record>(
        ConstructionShape::Named,
        fields,
        ConstructionProviders::from_runtime_providers(Some(provider), Some(provider)),
        ConstructionAvailability::Available,
    )
}

fn choice_variant_plan() -> ConstructionPlan {
    let fields = Box::leak(Box::new([
        ConstructField {
            name: "0",
            value: type_identity::<i32>,
            visibility: ConstructVisibility::Public,
            policy: ConstructionFieldPolicy::Required,
            update_policy: UpdateFieldPolicy::Preserved,
        },
        ConstructField {
            name: "1",
            value: type_identity::<i32>,
            visibility: ConstructVisibility::Public,
            policy: ConstructionFieldPolicy::Required,
            update_policy: UpdateFieldPolicy::Preserved,
        },
    ]));
    let provider = Box::leak(Box::new(PairProvider));
    ConstructionPlan::for_variant::<Choice>(
        1,
        ConstructionShape::Positional,
        fields,
        ConstructionProviders::from_runtime_providers(Some(provider), None),
        ConstructionAvailability::Available,
    )
}

fn marker_plan() -> ConstructionPlan {
    let provider = Box::leak(Box::new(MarkerProvider));
    ConstructionPlan::for_type::<Marker>(
        ConstructionShape::Unit,
        &[],
        ConstructionProviders::from_runtime_providers(Some(provider), None),
        ConstructionAvailability::Available,
    )
}

#[test]
fn construct_accepts_named_positional_and_unit_shapes() {
    let record_plan = record_plan();
    let record = ConstructionDescriptor::new(&record_plan)
        .construct::<Record, Local, _>(
            NamedFields::new()
                .field("count", 7_u32)
                .field("title", String::from("before")),
        )
        .unwrap();
    assert_eq!(
        record,
        Record {
            count: 7,
            title: "before".into(),
            secret: false
        }
    );

    let choice_plan = choice_variant_plan();
    let choice = ConstructionDescriptor::new(&choice_plan)
        .construct::<Choice, Local, _>(PositionalFields::new().value(2_i32).value(3_i32))
        .unwrap();
    assert!(matches!(choice, Choice::Coordinates(2, 3)));

    let marker_plan = marker_plan();
    let marker = ConstructionDescriptor::new(&marker_plan)
        .construct::<Marker, Local, _>(UnitFields)
        .unwrap();
    assert_eq!(marker, Marker);
}

#[test]
fn named_input_rejects_duplicates_and_recovers_every_entry() {
    let plan = record_plan();
    let failure = ConstructionDescriptor::new(&plan)
        .construct::<Record, Local, _>(NamedFields::new().field("count", 4_u32).field("count", 5_u32))
        .unwrap_err();
    assert_eq!(failure.error(), &ConstructionError::DuplicateField { field: "count" });
    let RecoveredConstructionInput::Named(input) = failure.into_input() else {
        panic!("named input shape was not retained");
    };
    assert_eq!(input.entries.len(), 2);
    assert_eq!(input.entries[0].1.downcast_ref::<u32>(), Some(&4));
    assert_eq!(input.entries[1].1.downcast_ref::<u32>(), Some(&5));
}

#[test]
fn construction_rejects_a_typed_target_mismatch_before_provider_execution() {
    let plan = record_plan();
    let failure = ConstructionDescriptor::new(&plan)
        .construct::<Wrong, Local, _>(NamedFields::new().field("count", 3_u32))
        .unwrap_err();
    assert!(matches!(failure.error(), ConstructionError::TargetTypeMismatch(_)));
    let RecoveredConstructionInput::Named(input) = failure.into_input() else {
        panic!("named input shape was not retained");
    };
    assert_eq!(input.entries.len(), 1);
    assert_eq!(input.entries[0].1.downcast_ref::<u32>(), Some(&3));
}

#[test]
fn shape_mismatch_keeps_the_original_input_shape_and_values() {
    let plan = choice_variant_plan();
    let failure = ConstructionDescriptor::new(&plan)
        .construct::<Choice, Local, _>(NamedFields::new().field("0", 11_i32))
        .unwrap_err();
    assert_eq!(
        failure.error(),
        &ConstructionError::ShapeMismatch {
            expected: ConstructionShape::Positional,
            actual: ConstructionShape::Named,
        }
    );
    assert_eq!(failure.recovery().input().shape(), ConstructionShape::Named);
}

#[test]
fn recoverable_failure_protocol_borrows_and_splits_construction_and_update_failures() {
    let plan = choice_variant_plan();
    let failure = ConstructionDescriptor::new(&plan)
        .construct::<Choice, Local, _>(NamedFields::new().field("0", 11_i32))
        .unwrap_err();
    assert!(matches!(
        RecoverableFailure::error(&failure),
        ConstructionError::ShapeMismatch { .. }
    ));
    assert_eq!(
        RecoverableFailure::recovery(&failure).input().shape(),
        ConstructionShape::Named
    );
    let (error, recovery) = RecoverableFailure::into_parts(failure);
    assert!(matches!(error, ConstructionError::ShapeMismatch { .. }));
    let RecoveredConstructionInput::Named(input) = recovery.into_input() else {
        panic!("construction recovery must retain its named input shape");
    };
    assert_eq!(input.entries[0].1.downcast_ref::<i32>(), Some(&11));

    let plan = record_plan_with_unavailable_update();
    let failure = match ConstructionDescriptor::new(&plan).update_dynamic::<Local>(
        UpdateInput::new(Record {
            count: 1,
            title: "old".into(),
            secret: false,
        })
        .field("title", String::from("new")),
    ) {
        Err(failure) => failure,
        Ok(_) => panic!("unavailable update policy must reject before consuming inputs"),
    };
    assert!(matches!(
        RecoverableFailure::error(&failure),
        UpdateError::UpdateUnavailable(UpdateUnavailableReason::NoProvider)
    ));
    assert_eq!(
        RecoverableFailure::recovery(&failure)
            .base()
            .downcast_ref::<Record>()
            .map(|record| record.count),
        Some(1)
    );
    let (error, recovery) = RecoverableFailure::into_parts(failure);
    assert!(matches!(
        error,
        UpdateError::UpdateUnavailable(UpdateUnavailableReason::NoProvider)
    ));
    let input = recovery.into_input();
    assert_eq!(input.replacements.entries.len(), 1);
}

#[test]
fn safe_visibility_and_unchecked_type_validation_are_separate() {
    let plan = record_plan_with_visibility(ConstructVisibility::Private);
    let safe_failure = ConstructionDescriptor::new(&plan)
        .construct::<Record, Local, _>(
            NamedFields::new()
                .field("count", 1_u32)
                .field("title", String::from("x"))
                .field("secret", true),
        )
        .unwrap_err();
    assert_eq!(
        safe_failure.error(),
        &ConstructionError::FieldNotPublic { field: "secret" }
    );

    let type_failure = unsafe {
        ConstructionDescriptor::new(&plan).construct_unchecked::<Record, Local, _>(
            NamedFields::new()
                .field("count", 1_u32)
                .field("title", String::from("x"))
                .field("secret", 5_u32),
        )
    }
    .unwrap_err();
    assert!(matches!(
        type_failure.error(),
        ConstructionError::ValueTypeMismatch { field: "secret", .. }
    ));
    let RecoveredConstructionInput::Named(input) = type_failure.into_input() else {
        panic!("named input shape was not retained");
    };
    assert_eq!(input.entries[2].1.downcast_ref::<u32>(), Some(&5));

    let value = unsafe {
        ConstructionDescriptor::new(&plan).construct_unchecked::<Record, Local, _>(
            NamedFields::new()
                .field("count", 1_u32)
                .field("title", String::from("x"))
                .field("secret", true),
        )
    }
    .unwrap();
    assert!(value.secret);
}

#[test]
fn private_required_fields_make_only_safe_construction_unavailable() {
    let plan = record_plan_with_policy(ConstructVisibility::Private, ConstructionFieldPolicy::Required);
    assert!(matches!(
        ConstructionDescriptor::new(&plan).availability(),
        ConstructionAvailability::Unavailable { reasons }
            if reasons.contains(&crate::runtime::construct::ConstructionUnavailableReason::PrivateRequiredField)
    ));
    let failure = ConstructionDescriptor::new(&plan)
        .construct::<Record, Local, _>(
            NamedFields::new()
                .field("count", 1_u32)
                .field("title", String::from("x"))
                .field("secret", true),
        )
        .unwrap_err();
    assert!(matches!(failure.error(), ConstructionError::Unavailable(_)));

    let record = unsafe {
        ConstructionDescriptor::new(&plan).construct_unchecked::<Record, Local, _>(
            NamedFields::new()
                .field("count", 1_u32)
                .field("title", String::from("x"))
                .field("secret", true),
        )
    }
    .unwrap();
    assert!(record.secret);
}

#[test]
fn update_unchecked_only_skips_visibility_and_still_checks_update_policy() {
    let plan = record_plan_with_visibility(ConstructVisibility::Private);
    let input = UpdateInput::new(Record {
        count: 1,
        title: "old".into(),
        secret: false,
    })
    .field("secret", true);
    let failure = unsafe { ConstructionDescriptor::new(&plan).update_unchecked::<Record, Local>(input) }.unwrap_err();
    assert_eq!(failure.error(), &UpdateError::FieldPreserved { field: "secret" });
    let input = failure.into_input();
    assert!(!input.base.downcast_ref::<Record>().unwrap().secret);
    assert_eq!(input.replacements.entries.len(), 1);
}

#[test]
fn unavailable_update_policy_rejects_the_whole_update_and_recovers_inputs() {
    let plan = record_plan_with_unavailable_update();
    let failure = match ConstructionDescriptor::new(&plan).update_dynamic::<Local>(
        UpdateInput::new(Record {
            count: 1,
            title: "old".into(),
            secret: false,
        })
        .field("title", String::from("new")),
    ) {
        Err(failure) => failure,
        Ok(_) => panic!("unavailable update policy unexpectedly invoked its provider"),
    };
    assert_eq!(
        failure.error(),
        &UpdateError::UpdateUnavailable(UpdateUnavailableReason::NoProvider)
    );
    let input = failure.into_input();
    assert_eq!(input.base.downcast_ref::<Record>().unwrap().count, 1);
    assert_eq!(input.replacements.entries.len(), 1);
}

#[test]
fn update_checks_all_inputs_before_consuming_base_and_replacements() {
    let plan = record_plan();
    let failure = ConstructionDescriptor::new(&plan)
        .update::<Record, Local>(
            UpdateInput::new(Record {
                count: 1,
                title: "old".into(),
                secret: false,
            })
            .field("count", 8_u32)
            .field("title", String::from("new"))
            .field("missing", true),
        )
        .unwrap_err();
    assert_eq!(
        failure.error(),
        &UpdateError::UnknownField {
            field: "missing".into()
        }
    );
    let input = failure.into_input();
    assert_eq!(input.base.downcast_ref::<Record>().unwrap().count, 1);
    assert_eq!(input.replacements.entries.len(), 3);

    let updated = ConstructionDescriptor::new(&plan)
        .update::<Record, Local>(
            UpdateInput::new(Record {
                count: 1,
                title: "old".into(),
                secret: false,
            })
            .field("count", 8_u32)
            .field("title", String::from("new")),
        )
        .unwrap();
    assert_eq!(
        updated,
        Record {
            count: 8,
            title: "new".into(),
            secret: false
        }
    );
}

#[test]
fn thread_safe_mode_uses_its_typed_provider() {
    let plan = record_plan();
    let record = ConstructionDescriptor::new(&plan)
        .construct::<Record, ThreadSafe, _>(
            NamedFields::<ThreadSafe>::new()
                .field("count", 9_u32)
                .field("title", String::from("thread")),
        )
        .unwrap();
    assert_eq!(record.count, 9);
}

#[test]
fn provider_lookup_reports_missing_and_ambiguous_metadata() {
    let plan = record_plan();
    assert!(matches!(
        resolve_plan(type_identity::<Wrong>(), std::slice::from_ref(&plan)),
        Err(ConstructionError::MissingProvider)
    ));
    let duplicate = record_plan();
    assert!(matches!(
        resolve_plan(type_identity::<Record>(), &[plan, duplicate]),
        Err(ConstructionError::AmbiguousProvider { count: 2, .. })
    ));

    let pair = choice_variant_plan();
    assert!(matches!(
        resolve_plan(type_identity::<Choice>(), std::slice::from_ref(&pair)),
        Err(ConstructionError::MissingProvider)
    ));
    assert!(resolve_variant_plan(type_identity::<Choice>(), 1, std::slice::from_ref(&pair)).is_ok());
    assert!(matches!(
        resolve_variant_plan(type_identity::<Choice>(), 0, std::slice::from_ref(&pair)),
        Err(ConstructionError::MissingProvider)
    ));
    let duplicate_variant = choice_variant_plan();
    assert!(matches!(
        resolve_variant_plan(type_identity::<Choice>(), 1, &[pair, duplicate_variant]),
        Err(ConstructionError::AmbiguousProvider { count: 2, .. })
    ));
}

#[test]
fn missing_provider_descriptor_reports_reason_and_recovers_input() {
    let target = ConstructionTarget::for_runtime(
        TypeIdentity {
            type_id: TypeId::of::<Record>(),
            rust_name: std::any::type_name::<Record>(),
        },
        ConstructionTargetKind::Struct,
        None,
    );
    let descriptor = ConstructionDescriptor::unavailable(
        target,
        ConstructionShape::Named,
        &[],
        crate::runtime::construct::ConstructionUnavailableReason::NoProvider,
    );

    assert!(matches!(
        descriptor.availability(),
        ConstructionAvailability::Unavailable { reasons }
            if reasons.as_ref() == [crate::runtime::construct::ConstructionUnavailableReason::NoProvider]
    ));

    let failure = descriptor
        .construct::<Record, Local, _>(NamedFields::new().field("count", 7_u32))
        .expect_err("a missing provider is reported before input is consumed");
    assert!(matches!(failure.error(), ConstructionError::Unavailable(_)));
    assert!(matches!(failure.into_input(), RecoveredConstructionInput::Named(_)));
}

#[test]
fn context_freezes_construction_plans_without_calling_providers() {
    FREEZE_PROVIDER_CALLS.store(0, Ordering::SeqCst);
    let context = crate::runtime::context::handle::builder::ReflectionContextBuilder::new()
        .register(
            crate::runtime::context::handle::registration::Registration::of::<Record>()
                .with_construction_plan(freeze_probe_plan()),
        )
        .build()
        .expect("construction plan context builds");
    let plans = context
        .type_of::<Record>()
        .construction_plans()
        .expect("construction plans are indexed");
    assert_eq!(plans.len(), 1);
    assert_eq!(FREEZE_PROVIDER_CALLS.load(Ordering::SeqCst), 0);
}
