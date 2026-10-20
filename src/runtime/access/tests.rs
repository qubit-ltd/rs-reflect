// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![allow(unsafe_code)]

use crate::runtime::access::DynamicFieldAdapter;
use crate::runtime::access::DynamicOnlyFieldAdapter;
use crate::runtime::access::FieldAdapter;
use crate::runtime::access::TypedFieldAdapter;
use crate::runtime::context::ReflectionContext;
use crate::runtime::descriptor::Reflect;
use crate::runtime::descriptor::TypeDescriptor;
use crate::runtime::descriptor::TypeRegistration;
use crate::runtime::descriptor::collections::TypeShapeRegistration;
use crate::runtime::descriptor::field::FieldAccess;
use crate::runtime::descriptor::field::FieldAccessError;
use crate::runtime::descriptor::field::FieldDescriptor;
use crate::runtime::descriptor::field::FieldMetadata;
use crate::runtime::descriptor::field::FieldSetError;
use crate::runtime::descriptor::field::Visibility;
use crate::runtime::descriptor::type_kind::TypeKindMetadata;
use crate::runtime::descriptor::variant::DiscriminantOrigin;
use crate::runtime::descriptor::variant::NumericDiscriminant;
use crate::runtime::descriptor::variant::TypedVariantAdapter;
use crate::runtime::descriptor::variant::VariantDescriptor;
use crate::runtime::descriptor::variant::VariantDiscriminant;
use crate::runtime::descriptor::variant::VariantIdentity;
use crate::runtime::descriptor::variant::VariantMetadata;
use crate::runtime::descriptor::variant::VariantShape;
use crate::runtime::expression::ExpressionData;
use crate::runtime::value::DynamicMut;
use crate::runtime::value::DynamicOwned;
use crate::runtime::value::DynamicRef;
use crate::runtime::value::Local;
use crate::runtime::value::ThreadSafe;

fn slice_ref(value: &[u8]) -> &[u8] {
    value
}

fn slice_mut(value: &mut [u8]) -> &mut [u8] {
    value
}

static DST_ADAPTER: DynamicOnlyFieldAdapter<[u8], [u8]> = DynamicOnlyFieldAdapter::new(slice_ref, slice_mut);

struct Record {
    id: u32,
}

impl Reflect for Record {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        static REGISTRATION: TypeRegistration<Record> = TypeRegistration::new();
        &REGISTRATION
    }
}

fn get_id(record: &Record) -> &u32 {
    &record.id
}

fn get_id_mut(record: &mut Record) -> &mut u32 {
    &mut record.id
}

static ID_ADAPTER: TypedFieldAdapter<Record, u32> = TypedFieldAdapter::new(get_id, get_id_mut);
static ID_EXPRESSION: ExpressionData = ExpressionData::Never;
static ID_FIELD: FieldMetadata = FieldMetadata::new(
    0,
    "id",
    Visibility::Public,
    &ID_EXPRESSION,
    &ID_ADAPTER,
    None,
    None,
    FieldAccess {
        readable: true,
        mutable: true,
        replaceable: true,
        constructible: true,
    },
)
.with_dynamic_adapters(Some(ID_ADAPTER.local_dynamic()), Some(ID_ADAPTER.thread_safe_dynamic()));
static RECORD_FIELDS: [FieldMetadata; 1] = [FieldMetadata::new(
    0,
    "id",
    Visibility::Public,
    &ID_EXPRESSION,
    &ID_ADAPTER,
    None,
    None,
    FieldAccess {
        readable: true,
        mutable: true,
        replaceable: true,
        constructible: true,
    },
)
.with_dynamic_adapters(Some(ID_ADAPTER.local_dynamic()), Some(ID_ADAPTER.thread_safe_dynamic()))];
static UNREADABLE_FIELD: FieldMetadata = FieldMetadata::new(
    0,
    "id",
    Visibility::Public,
    &ID_EXPRESSION,
    &ID_ADAPTER,
    None,
    None,
    FieldAccess {
        readable: false,
        mutable: true,
        replaceable: true,
        constructible: true,
    },
)
.with_dynamic_adapters(Some(ID_ADAPTER.local_dynamic()), Some(ID_ADAPTER.thread_safe_dynamic()));
static IMMUTABLE_FIELD: FieldMetadata = FieldMetadata::new(
    0,
    "id",
    Visibility::Public,
    &ID_EXPRESSION,
    &ID_ADAPTER,
    None,
    None,
    FieldAccess {
        readable: true,
        mutable: false,
        replaceable: true,
        constructible: true,
    },
)
.with_dynamic_adapters(Some(ID_ADAPTER.local_dynamic()), Some(ID_ADAPTER.thread_safe_dynamic()));
static NON_REPLACEABLE_FIELD: FieldMetadata = FieldMetadata::new(
    0,
    "id",
    Visibility::Public,
    &ID_EXPRESSION,
    &ID_ADAPTER,
    None,
    None,
    FieldAccess {
        readable: true,
        mutable: true,
        replaceable: false,
        constructible: false,
    },
)
.with_dynamic_adapters(Some(ID_ADAPTER.local_dynamic()), Some(ID_ADAPTER.thread_safe_dynamic()));
static NONPUBLIC_FIELDS: [FieldMetadata; 2] = [
    FieldMetadata::new(
        0,
        "id",
        Visibility::Crate,
        &ID_EXPRESSION,
        &ID_ADAPTER,
        None,
        None,
        FieldAccess::new(true, true, true, true),
    )
    .with_dynamic_adapters(Some(ID_ADAPTER.local_dynamic()), Some(ID_ADAPTER.thread_safe_dynamic())),
    FieldMetadata::new(
        0,
        "id",
        Visibility::Restricted("crate::model"),
        &ID_EXPRESSION,
        &ID_ADAPTER,
        None,
        None,
        FieldAccess::new(true, true, true, true),
    )
    .with_dynamic_adapters(Some(ID_ADAPTER.local_dynamic()), Some(ID_ADAPTER.thread_safe_dynamic())),
];
static RECORD_KIND: TypeKindMetadata = TypeKindMetadata::Struct(&RECORD_FIELDS);
inventory::submit! {
    TypeShapeRegistration::new::<Record>(&RECORD_KIND)
}

enum Choice {
    Count(u32),
    Nothing,
}

impl Reflect for Choice {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        static REGISTRATION: TypeRegistration<Choice> = TypeRegistration::new();
        &REGISTRATION
    }
}

fn get_count(choice: &Choice) -> &u32 {
    let Choice::Count(value) = choice else {
        unreachable!("the field adapter runs only for the active variant")
    };
    value
}

fn get_count_mut(choice: &mut Choice) -> &mut u32 {
    let Choice::Count(value) = choice else {
        unreachable!("the field adapter runs only for the active variant")
    };
    value
}

fn is_count(choice: &Choice) -> bool {
    matches!(choice, Choice::Count(_))
}

fn active_choice(choice: &Choice) -> VariantIdentity {
    match choice {
        Choice::Count(_) => COUNT_VARIANT.identity(),
        Choice::Nothing => NOTHING_VARIANT.identity(),
    }
}

static COUNT_ADAPTER: TypedVariantAdapter<Choice> = TypedVariantAdapter::new(is_count);
static NOTHING_ADAPTER: TypedVariantAdapter<Choice> =
    TypedVariantAdapter::new(|choice| matches!(choice, Choice::Nothing));
static COUNT_FIELDS: [FieldMetadata; 0] = [];
static NOTHING_FIELDS: [FieldMetadata; 0] = [];
static COUNT_VARIANT: VariantMetadata = VariantMetadata::new(
    0,
    "Count",
    VariantShape::Tuple,
    Visibility::Public,
    VariantDiscriminant {
        origin: DiscriminantOrigin::Explicit,
        numeric: Some(NumericDiscriminant::U8(7)),
    },
    &COUNT_FIELDS,
    &COUNT_ADAPTER,
);
static NOTHING_VARIANT: VariantMetadata = VariantMetadata::new(
    1,
    "Nothing",
    VariantShape::Unit,
    Visibility::Public,
    VariantDiscriminant {
        origin: DiscriminantOrigin::Implicit,
        numeric: None,
    },
    &NOTHING_FIELDS,
    &NOTHING_ADAPTER,
);
static CHOICE_VARIANTS: [VariantMetadata; 2] = [
    VariantMetadata::new(
        0,
        "Count",
        VariantShape::Tuple,
        Visibility::Public,
        VariantDiscriminant {
            origin: DiscriminantOrigin::Explicit,
            numeric: Some(NumericDiscriminant::U8(7)),
        },
        &COUNT_FIELDS,
        &COUNT_ADAPTER,
    ),
    VariantMetadata::new(
        1,
        "Nothing",
        VariantShape::Unit,
        Visibility::Public,
        VariantDiscriminant {
            origin: DiscriminantOrigin::Implicit,
            numeric: None,
        },
        &NOTHING_FIELDS,
        &NOTHING_ADAPTER,
    ),
];
static CHOICE_KIND: TypeKindMetadata = TypeKindMetadata::Enum(&CHOICE_VARIANTS);
inventory::submit! {
    TypeShapeRegistration::new::<Choice>(&CHOICE_KIND)
}

unsafe fn active_choice_variant(value: *const ()) -> VariantIdentity {
    // SAFETY: FieldDescriptor checks the Choice owner TypeId before calling this
    // adapter.
    match unsafe { &*value.cast::<Choice>() } {
        Choice::Count(_) => COUNT_VARIANT.identity(),
        Choice::Nothing => NOTHING_VARIANT.identity(),
    }
}

fn choice_count_ref(choice: &Choice) -> &u32 {
    get_count(choice)
}

fn choice_count_mut(choice: &mut Choice) -> &mut u32 {
    get_count_mut(choice)
}

static COUNT_FIELD_ADAPTER: TypedFieldAdapter<Choice, u32> =
    TypedFieldAdapter::with_variant(choice_count_ref, choice_count_mut, active_choice);
static COUNT_EXPRESSION: ExpressionData = ExpressionData::Never;
static COUNT_FIELD: FieldMetadata = FieldMetadata::new(
    0,
    "0",
    Visibility::Public,
    &COUNT_EXPRESSION,
    &COUNT_FIELD_ADAPTER,
    Some(&COUNT_VARIANT),
    Some(active_choice_variant),
    FieldAccess {
        readable: true,
        mutable: true,
        replaceable: true,
        constructible: true,
    },
)
.with_dynamic_adapters(
    Some(COUNT_FIELD_ADAPTER.local_dynamic()),
    Some(COUNT_FIELD_ADAPTER.thread_safe_dynamic()),
);

fn descriptor<'ctx>(context: &'ctx ReflectionContext) -> FieldDescriptor<'ctx> {
    let ty = TypeDescriptor::of_in::<Record>(context);
    FieldDescriptor::new(&ID_FIELD, ty, None)
}

#[test]
fn typed_field_access_reads_mutates_and_replaces() {
    let context = ReflectionContext::empty();
    let field = descriptor(&context);
    let mut record = Record { id: 11 };

    assert_eq!(*field.get::<Record, u32>(&record).expect("field is readable"), 11);
    *field.get_mut::<Record, u32>(&mut record).expect("field is mutable") = 12;
    field.set::<Record, u32>(&mut record, 13).expect("field is replaceable");
    assert_eq!(record.id, 13);
}

#[test]
fn typed_field_access_rejects_wrong_target_and_value_types() {
    let context = ReflectionContext::empty();
    let field = descriptor(&context);
    let wrong_target = 3_u64;
    assert!(matches!(
        field.get::<u64, u32>(&wrong_target),
        Err(FieldAccessError::TargetTypeMismatch(_))
    ));

    let record = Record { id: 11 };
    assert!(matches!(
        field.get::<Record, u64>(&record),
        Err(FieldAccessError::ValueTypeMismatch(_))
    ));
}

#[test]
fn failed_replacement_returns_original_value_without_mutating_target() {
    let context = ReflectionContext::empty();
    let field = descriptor(&context);
    let mut wrong_target = 4_u64;
    let Err(failure) = field.set::<u64, String>(&mut wrong_target, String::from("keep me")) else {
        panic!("target identity must match the adapter");
    };
    let (error, replacement) = failure.into_parts();

    assert!(matches!(error, FieldSetError::TargetTypeMismatch(_)));
    assert_eq!(replacement, "keep me");
    assert_eq!(wrong_target, 4);

    let mut record = Record { id: 11 };
    let Err(failure) = field.set::<Record, String>(&mut record, String::from("retry")) else {
        panic!("field type mismatch must preserve the replacement");
    };
    let (error, replacement) = failure.into_parts();
    assert!(matches!(error, FieldSetError::ValueTypeMismatch(_)));
    assert_eq!(replacement, "retry");
    assert_eq!(record.id, 11);
}

#[test]
fn inaccessible_field_is_described_but_not_read() {
    let context = ReflectionContext::empty();
    let ty = TypeDescriptor::of_in::<Record>(&context);
    static PRIVATE_FIELD: FieldMetadata = FieldMetadata::new(
        0,
        "id",
        Visibility::Private,
        &ID_EXPRESSION,
        &ID_ADAPTER,
        None,
        None,
        FieldAccess {
            readable: true,
            mutable: true,
            replaceable: true,
            constructible: true,
        },
    );
    let field = FieldDescriptor::new(&PRIVATE_FIELD, ty, None);
    let mut record = Record { id: 1 };
    assert!(matches!(
        field.get::<Record, u32>(&record),
        Err(FieldAccessError::NotPublic)
    ));
    // SAFETY: this test owns the value and explicitly exercises the unsafe
    // visibility bypass.
    assert_eq!(
        *unsafe { field.get_unsafe::<Record, u32>(&record) }.expect("unsafe bypass"),
        1
    );
    *unsafe { field.get_mut_unsafe::<Record, u32>(&mut record) }.expect("unsafe mutable bypass") = 2;
    // SAFETY: this test owns the value and explicitly exercises the unsafe
    // visibility bypass.
    unsafe { field.set_unsafe::<Record, u32>(&mut record, 3) }.expect("unsafe replacement bypass");
    assert_eq!(record.id, 3);
}

#[test]
fn context_shape_registrations_drive_descriptor_collections() {
    let context = ReflectionContext::builder()
        .include_type::<Record>()
        .include_type::<Choice>()
        .build()
        .expect("context includes descriptor shape registrations");
    let record = TypeDescriptor::of_in::<Record>(&context);
    let fields = record.fields();
    assert_eq!(fields.len(), 1);
    assert_eq!(fields.iter().len(), 1);
    assert_eq!(fields.get_at(0).map(FieldDescriptor::name), Some("id"));
    assert!(fields.get_at(1).is_none());
    assert!(fields.get("missing").is_none());
    assert_eq!(record.field("id").expect("strict field").name(), "id");
    assert!(record.field("missing").is_err());
    assert!(record.variants().is_empty());

    let choice = TypeDescriptor::of_in::<Choice>(&context);
    let variants = choice.variants();
    assert_eq!(variants.len(), 2);
    assert_eq!(variants.iter().len(), 2);
    assert_eq!(variants.get_at(0).map(VariantDescriptor::name), Some("Count"));
    assert_eq!(variants.get_at(1).map(VariantDescriptor::name), Some("Nothing"));
    assert!(variants.get_at(2).is_none());
    assert!(variants.get("missing").is_none());
    assert_eq!(choice.variant("Count").expect("strict variant").name(), "Count");
    assert!(choice.variant("missing").is_err());
    assert!(choice.fields().is_empty());
}

#[test]
fn dynamically_sized_adapter_keeps_dst_out_of_typed_pointer_access() {
    assert_eq!(DST_ADAPTER.owner().type_id, std::any::TypeId::of::<[u8]>());
    assert_eq!(DST_ADAPTER.value().type_id, std::any::TypeId::of::<[u8]>());
    // SAFETY: this adapter is deliberately dynamic-only and never dereferences raw
    // pointers.
    assert!(unsafe { DST_ADAPTER.ref_ptr(std::ptr::null()) }.is_none());
    // SAFETY: this adapter is deliberately dynamic-only and never dereferences raw
    // pointers.
    assert!(unsafe { DST_ADAPTER.mut_ptr(std::ptr::null_mut()) }.is_none());
}

#[test]
fn variant_activity_and_enum_field_access_follow_active_variant() {
    let context = ReflectionContext::empty();
    let ty = TypeDescriptor::of_in::<Choice>(&context);
    let count = VariantDescriptor::new(&COUNT_VARIANT, ty);
    let nothing = VariantDescriptor::new(&NOTHING_VARIANT, ty);
    let count_value = Choice::Count(8);
    let nothing_value = Choice::Nothing;

    assert!(count.is_active(&count_value).expect("same enum type"));
    assert!(!count.is_active(&nothing_value).expect("same enum type"));
    assert!(
        count
            .is_active_dynamic(DynamicRef::<Local>::new(&count_value))
            .expect("dynamic value has the enum type")
    );
    assert!(
        !count
            .is_active_dynamic(DynamicRef::<Local>::new(&nothing_value))
            .expect("dynamic value has the enum type")
    );
    assert!(
        count
            .is_active_dynamic(DynamicRef::<Local>::new(&Record { id: 1 }))
            .is_err()
    );
    assert!(count.is_active(&8_u32).is_err());
    assert_eq!(count.discriminant().numeric, Some(NumericDiscriminant::U8(7)));
    assert_eq!(nothing.shape(), VariantShape::Unit);

    let field = FieldDescriptor::new(&COUNT_FIELD, ty, Some(count));
    assert_eq!(*field.get::<Choice, u32>(&count_value).expect("active field"), 8);
    assert!(matches!(
        field.get::<Choice, u32>(&nothing_value),
        Err(FieldAccessError::InactiveVariant { .. })
    ));
}

#[test]
fn dynamic_field_access_preserves_mode_and_replacement_on_failure() {
    let context = ReflectionContext::empty();
    let field = descriptor(&context);
    let record = Record { id: 21 };
    let value = field
        .get_dynamic(DynamicRef::<Local>::new(&record))
        .expect("local adapter is registered");
    assert_eq!(*value.downcast_ref::<u32>().expect("field type"), 21);

    let mut record = Record { id: 22 };
    let value = field
        .get_dynamic_mut(DynamicMut::<Local>::new(&mut record))
        .expect("local mutable adapter is registered");
    *value.downcast::<u32>().expect("field type") = 23;
    assert_eq!(record.id, 23);

    let mut record = Record { id: 24 };
    let failure = field
        .set_dynamic(
            DynamicMut::<Local>::new(&mut record),
            DynamicOwned::<Local>::new(String::from("recover")),
        )
        .expect_err("field value type must match");
    let (error, replacement) = failure.into_parts();
    assert!(matches!(error, FieldSetError::ValueTypeMismatch(_)));
    assert_eq!(
        replacement
            .downcast::<String>()
            .unwrap_or_else(|_| panic!("value recovery")),
        "recover"
    );
    assert_eq!(record.id, 24);

    let mut record = Record { id: 25 };
    field
        .set_dynamic(
            DynamicMut::<ThreadSafe>::new(&mut record),
            DynamicOwned::<ThreadSafe>::new(26_u32),
        )
        .expect("thread-safe adapter preserves the mode");
    assert_eq!(record.id, 26);
}

#[test]
fn dynamic_field_adapter_rejects_wrong_target_and_returns_replacement() {
    let context = ReflectionContext::empty();
    let field = descriptor(&context);
    let wrong_target = 27_u64;
    assert!(matches!(
        field.get_dynamic(DynamicRef::<Local>::new(&wrong_target)),
        Err(FieldAccessError::TargetTypeMismatch(_))
    ));
    let mut wrong_mut_target = 28_u64;
    assert!(matches!(
        field.get_dynamic_mut(DynamicMut::<Local>::new(&mut wrong_mut_target)),
        Err(FieldAccessError::TargetTypeMismatch(_))
    ));

    let mut wrong_target = 29_u64;
    let failure = field
        .set_dynamic(
            DynamicMut::<Local>::new(&mut wrong_target),
            DynamicOwned::<Local>::new(String::from("target retry")),
        )
        .expect_err("the adapter owner type must match the dynamic target");
    assert!(matches!(failure.error(), FieldSetError::TargetTypeMismatch(_)));
    assert_eq!(
        failure
            .replacement()
            .downcast_ref::<String>()
            .expect("failure retains replacement"),
        "target retry"
    );
    assert_eq!(wrong_target, 29);
}

#[test]
fn missing_dynamic_adapter_is_reported_and_preserves_replacement() {
    static FIELD_WITHOUT_ADAPTER: FieldMetadata = FieldMetadata::new(
        0,
        "id",
        Visibility::Public,
        &ID_EXPRESSION,
        &ID_ADAPTER,
        None,
        None,
        FieldAccess {
            readable: true,
            mutable: true,
            replaceable: true,
            constructible: true,
        },
    );
    let context = ReflectionContext::empty();
    let ty = TypeDescriptor::of_in::<Record>(&context);
    let field = FieldDescriptor::new(&FIELD_WITHOUT_ADAPTER, ty, None);
    let record = Record { id: 31 };

    assert_eq!(
        field.access(),
        FieldAccess {
            readable: false,
            mutable: false,
            replaceable: false,
            constructible: false,
        }
    );
    assert_eq!(
        *field
            .get::<Record, u32>(&record)
            .expect("typed access does not require a dynamic adapter"),
        31
    );
    assert!(matches!(
        field.get_dynamic(DynamicRef::<Local>::new(&record)),
        Err(FieldAccessError::Unavailable(_))
    ));
    assert!(matches!(
        field.get_dynamic_mut(DynamicMut::<Local>::new(&mut Record { id: 31 })),
        Err(FieldAccessError::Unavailable(_))
    ));
    assert!(matches!(
        field.get_dynamic::<ThreadSafe>(DynamicRef::<ThreadSafe>::new(&record)),
        Err(FieldAccessError::Unavailable(_))
    ));

    let mut record = Record { id: 32 };
    let Err(failure) = field.set_dynamic(
        DynamicMut::<Local>::new(&mut record),
        DynamicOwned::<Local>::new(String::from("recover")),
    ) else {
        panic!("missing adapter must fail");
    };
    let (error, replacement) = failure.into_parts();
    assert!(matches!(error, FieldSetError::Unavailable(_)));
    assert_eq!(
        replacement
            .downcast::<String>()
            .unwrap_or_else(|_| panic!("value recovery")),
        "recover"
    );
    assert_eq!(record.id, 32);
}

#[test]
fn dynamic_only_dst_adapter_reads_and_mutates_but_recovers_unsupported_assignment() {
    let bytes = [1_u8, 2, 3];
    assert!(DynamicFieldAdapter::<Local>::can_get(&DST_ADAPTER));
    assert!(DynamicFieldAdapter::<Local>::can_get_mut(&DST_ADAPTER));
    assert!(!DynamicFieldAdapter::<Local>::can_set(&DST_ADAPTER));
    let value = DynamicFieldAdapter::<Local>::get(&DST_ADAPTER, DynamicRef::<Local>::new(&bytes[..]), None)
        .expect("dynamic-only adapter supports a shared DST borrow");
    assert_eq!(value.downcast_ref::<[u8]>(), Some(&bytes[..]));

    let mut bytes = [4_u8, 5, 6];
    let value = DynamicFieldAdapter::<Local>::get_mut(&DST_ADAPTER, DynamicMut::<Local>::new(&mut bytes[..]), None)
        .expect("dynamic-only adapter supports an exclusive DST borrow");
    value.downcast::<[u8]>().expect("DST type")[1] = 9;
    assert_eq!(bytes, [4, 9, 6]);

    let mut bytes = [7_u8, 8];
    let failure = DynamicFieldAdapter::<Local>::set(
        &DST_ADAPTER,
        DynamicMut::<Local>::new(&mut bytes[..]),
        DynamicOwned::<Local>::new(String::from("preserve")),
        None,
    )
    .expect_err("a dynamically sized field has no owned replacement adapter");
    let (error, replacement) = failure.into_parts();
    assert!(matches!(error, FieldSetError::Unavailable(_)));
    assert_eq!(replacement.downcast::<String>().expect("recovered value"), "preserve");
}

#[test]
fn field_policy_blocks_only_the_disabled_dynamic_operations() {
    let context = ReflectionContext::empty();
    let ty = TypeDescriptor::of_in::<Record>(&context);
    let record = Record { id: 41 };
    let unreadable = FieldDescriptor::new(&UNREADABLE_FIELD, ty, None);
    assert_eq!(unreadable.access(), FieldAccess::new(false, true, true, true));
    assert!(matches!(
        unreadable.get_dynamic(DynamicRef::<Local>::new(&record)),
        Err(FieldAccessError::NotReadable)
    ));

    let immutable = FieldDescriptor::new(&IMMUTABLE_FIELD, ty, None);
    assert_eq!(immutable.access(), FieldAccess::new(true, false, true, true));
    let mut record = Record { id: 42 };
    assert!(matches!(
        immutable.get_dynamic_mut(DynamicMut::<Local>::new(&mut record)),
        Err(FieldAccessError::NotMutable)
    ));
    immutable
        .set_dynamic(
            DynamicMut::<Local>::new(&mut record),
            DynamicOwned::<Local>::new(43_u32),
        )
        .expect("replacement policy is independent of mutable-borrow policy");
    assert_eq!(record.id, 43);

    let non_replaceable = FieldDescriptor::new(&NON_REPLACEABLE_FIELD, ty, None);
    assert_eq!(non_replaceable.access(), FieldAccess::new(true, true, false, false));
    let mut record = Record { id: 44 };
    let failure = non_replaceable
        .set_dynamic(
            DynamicMut::<Local>::new(&mut record),
            DynamicOwned::<Local>::new(String::from("retry")),
        )
        .expect_err("replacement is disabled by field policy");
    assert!(matches!(failure.error(), FieldSetError::NotReplaceable));
    assert_eq!(
        failure
            .replacement()
            .downcast_ref::<String>()
            .expect("failure retains replacement"),
        "retry"
    );
    assert_eq!(record.id, 44);
}

#[test]
fn all_non_public_visibility_kinds_deny_safe_access_and_preserve_replacements() {
    let context = ReflectionContext::empty();
    let ty = TypeDescriptor::of_in::<Record>(&context);
    let record = Record { id: 51 };

    for metadata in &NONPUBLIC_FIELDS {
        let field = FieldDescriptor::new(metadata, ty, None);
        assert_ne!(field.visibility(), Visibility::Public);
        assert!(matches!(
            field.get::<Record, u32>(&record),
            Err(FieldAccessError::NotPublic)
        ));
        assert!(matches!(
            field.get_dynamic(DynamicRef::<Local>::new(&record)),
            Err(FieldAccessError::NotPublic)
        ));

        let mut record = Record { id: 52 };
        let failure = field
            .set_dynamic(
                DynamicMut::<Local>::new(&mut record),
                DynamicOwned::<Local>::new(String::from("private")),
            )
            .expect_err("safe replacement must honor source visibility");
        assert!(matches!(failure.error(), FieldSetError::NotPublic));
        assert_eq!(
            failure
                .replacement()
                .downcast_ref::<String>()
                .expect("failure retains replacement"),
            "private"
        );
        assert_eq!(record.id, 52);
    }
}

#[test]
fn enum_variant_dynamic_access_checks_activity_and_recovers_replacements() {
    let context = ReflectionContext::empty();
    let ty = TypeDescriptor::of_in::<Choice>(&context);
    let field = FieldDescriptor::new(&COUNT_FIELD, ty, Some(VariantDescriptor::new(&COUNT_VARIANT, ty)));

    let mut inactive = Choice::Nothing;
    assert!(matches!(
        field.get_dynamic(DynamicRef::<Local>::new(&inactive)),
        Err(FieldAccessError::InactiveVariant { .. })
    ));
    assert!(matches!(
        field.get_dynamic_mut(DynamicMut::<Local>::new(&mut inactive)),
        Err(FieldAccessError::InactiveVariant { .. })
    ));
    let failure = field
        .set_dynamic(
            DynamicMut::<Local>::new(&mut inactive),
            DynamicOwned::<Local>::new(String::from("variant retry")),
        )
        .expect_err("inactive variant fields cannot be replaced");
    assert!(matches!(failure.error(), FieldSetError::InactiveVariant { .. }));
    assert_eq!(
        failure
            .replacement()
            .downcast_ref::<String>()
            .expect("failure retains replacement"),
        "variant retry"
    );
    assert!(matches!(inactive, Choice::Nothing));

    let mut active = Choice::Count(61);
    *field
        .get_dynamic_mut(DynamicMut::<Local>::new(&mut active))
        .expect("active variant exposes its field")
        .downcast::<u32>()
        .expect("field type") = 62;
    field
        .set_dynamic(
            DynamicMut::<Local>::new(&mut active),
            DynamicOwned::<Local>::new(63_u32),
        )
        .expect("active variant field is replaceable");
    assert!(matches!(active, Choice::Count(63)));
}
