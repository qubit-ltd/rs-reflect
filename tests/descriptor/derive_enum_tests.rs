// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// =============================================================================

//! Integration tests for vNext `Reflect` enum metadata.

use qubit_reflect::Reflect;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::descriptor::FieldAccessError;
use qubit_reflect::descriptor::FieldSetError;
use qubit_reflect::descriptor::TypeKind;
use qubit_reflect::value::DynamicRef;
use qubit_reflect::value::Local;

#[Reflect]
pub enum Event {
    Ready,
    Number(u32),
    Named {
        value: u8,
    },
    #[expect(dead_code, reason = "the reflected descriptor intentionally omits this variant")]
    #[reflect(skip)]
    Hidden(String),
}

#[Reflect]
enum SparseVariants {
    First,
    #[expect(dead_code, reason = "the reflected descriptor intentionally omits this variant")]
    #[reflect(skip)]
    Hidden,
    Last,
}

#[Reflect]
pub enum CfgTupleFields {
    DisabledMiddle(#[cfg(any())] MissingCfgFieldType, u16),
    EnabledMiddle(u8, #[cfg(not(any()))] u16, u32),
}

/// Verifies enum variants and their fields retain source names and positions.
#[test]
fn test_derive_enum_preserves_variant_and_field_metadata() {
    let descriptor = TypeDescriptor::of::<Event>();

    assert!(matches!(descriptor.kind(), TypeKind::Enum(_)));
    assert_eq!(descriptor.variants().len(), 3);
    assert_eq!(descriptor.variant("Ready").unwrap().index(), 0);
    assert_eq!(descriptor.variant("Number").unwrap().index(), 1);
    assert_eq!(
        descriptor
            .variant("Named")
            .unwrap()
            .fields()
            .get("value")
            .unwrap()
            .name(),
        "value"
    );
    assert!(descriptor.variants().get("Hidden").is_none());
    assert!(descriptor.variants().get_at(3).is_none());
}

#[test]
fn test_derive_enum_get_at_uses_source_variant_index() {
    let variants = TypeDescriptor::of::<SparseVariants>().variants();

    assert_eq!(variants.len(), 2);
    assert_eq!(variants.get_at(0).expect("First is source variant 0").name(), "First");
    assert!(variants.get_at(1).is_none(), "skipped source variant 1 is absent");
    let last = variants.get_at(2).expect("Last remains source variant 2");
    assert_eq!(last.name(), "Last");
    assert_eq!(last.index(), 2);
}

#[test]
fn test_tuple_variant_field_adapters_follow_cfg_compiled_shape() {
    let descriptor = TypeDescriptor::of::<CfgTupleFields>();
    let disabled_middle = descriptor
        .variant("DisabledMiddle")
        .expect("variant with a cfg-disabled middle field is described");
    let disabled_field = disabled_middle
        .field("1")
        .expect("the retained tuple field keeps its source index");
    assert_eq!(
        *disabled_field
            .get::<_, u16>(&CfgTupleFields::DisabledMiddle(19))
            .unwrap(),
        19
    );

    let enabled_middle = descriptor
        .variant("EnabledMiddle")
        .expect("variant with a cfg-enabled middle field is described");
    let enabled_field = enabled_middle
        .field("2")
        .expect("the trailing tuple field keeps its source index");
    assert_eq!(
        *enabled_field
            .get::<_, u32>(&CfgTupleFields::EnabledMiddle(3, 5, 23))
            .unwrap(),
        23
    );
}

/// Verifies a variant adapter accepts only its exact active variant.
#[test]
fn test_derive_enum_variant_activity_checks_typed_and_dynamic_values() {
    let descriptor = TypeDescriptor::of::<Event>();
    let ready = descriptor.variant("Ready").expect("Ready variant is described");
    let number = descriptor.variant("Number").expect("Number variant is described");

    assert!(ready.is_active(&Event::Ready).unwrap());
    assert!(!ready.is_active(&Event::Number(9)).unwrap());
    assert!(
        number
            .is_active_dynamic(DynamicRef::<Local>::new(&Event::Number(9)))
            .unwrap()
    );
    assert!(
        !number
            .is_active_dynamic(DynamicRef::<Local>::new(&Event::Ready))
            .unwrap()
    );
}

/// Verifies typed field access reports inactive variants before reaching the
/// field adapter and preserves a rejected replacement.
#[test]
fn test_derive_enum_field_access_preserves_inactive_variant_recovery() {
    let field = TypeDescriptor::of::<Event>()
        .variant("Number")
        .expect("Number variant is described")
        .field("0")
        .expect("tuple field is described");
    let mut value = Event::Ready;

    assert!(matches!(
        field.get::<_, u32>(&value),
        Err(FieldAccessError::InactiveVariant { .. })
    ));
    assert!(matches!(
        field.get::<_, u32>(&Event::Hidden(String::from("hidden"))),
        Err(FieldAccessError::InactiveVariant { .. })
    ));
    let failure = field
        .set(&mut value, 12_u32)
        .expect_err("setting an inactive variant field must fail");
    assert!(matches!(failure.error(), FieldSetError::InactiveVariant { .. }));
    assert_eq!(failure.into_parts().1, 12_u32);
}
