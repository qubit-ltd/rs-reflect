// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::construct::NamedConstructionInput;
use crate::construct::TupleConstructionInput;
use crate::descriptor::DiscriminantOrigin;
use crate::descriptor::NumericDiscriminant;
use crate::descriptor::TypeDescriptor;
use crate::descriptor::VariantDescriptor;
use crate::descriptor::VariantKind;
use crate::error::TypeMismatch;
use crate::value::DynamicRef;
use crate::value::Local;

#[cfg(feature = "derive")]
#[crate::Reflect]
enum VariantProbe {
    Ready,
    Waiting,
}

fn declaring_type() -> &'static TypeDescriptor {
    TypeDescriptor::of::<u8>()
}

fn active_test(value: DynamicRef<'_, Local>) -> Result<bool, TypeMismatch> {
    Ok(value.downcast_ref::<u8>().is_some())
}

#[test]
fn test_variant_descriptor_exposes_source_facts_and_unavailable_construction() {
    let variant = VariantDescriptor::new(declaring_type, 2, "Ready", "ready", VariantKind::Unit, &[], active_test)
        .with_discriminant(DiscriminantOrigin::Explicit, Some(NumericDiscriminant::U8(9)));

    assert!(std::ptr::eq(variant.declaring_type(), declaring_type()));
    assert_eq!(variant.index(), 2);
    assert_eq!(variant.rust_name(), "Ready");
    assert_eq!(variant.query_name(), "ready");
    assert_eq!(variant.kind(), VariantKind::Unit);
    assert_eq!(variant.discriminant_origin(), DiscriminantOrigin::Explicit);
    assert_eq!(variant.numeric_discriminant(), Some(NumericDiscriminant::U8(9)));
    assert!(variant.fields().is_empty());
    assert!(variant.field("missing").is_none());
    assert!(variant.field_at(0).is_none());
    assert!(variant.construction().is_none());

    let named = NamedConstructionInput::<Local>::new();
    let tuple = TupleConstructionInput::<Local>::new();
    assert!(variant.construct_struct::<u8>(named).is_err());
    assert!(variant.construct_tuple::<u8>(tuple).is_err());
    assert!(variant.construct_unit::<u8>().is_err());
}

#[test]
#[cfg(feature = "derive")]
fn test_variant_descriptor_checks_dynamic_active_value_type() {
    let descriptor = TypeDescriptor::of::<VariantProbe>();
    let ready = descriptor.variant("Ready").expect("Ready variant");
    let waiting = descriptor.variant("Waiting").expect("Waiting variant");

    assert!(
        ready
            .is_active_dynamic(DynamicRef::<Local>::new(&VariantProbe::Ready))
            .unwrap()
    );
    assert!(
        !ready
            .is_active_dynamic(DynamicRef::<Local>::new(&VariantProbe::Waiting))
            .unwrap()
    );
    assert!(
        waiting
            .is_active_dynamic(DynamicRef::<Local>::new(&VariantProbe::Waiting))
            .unwrap()
    );
    assert!(
        !waiting
            .is_active_dynamic(DynamicRef::<Local>::new(&VariantProbe::Ready))
            .unwrap()
    );
    let error = ready
        .is_active_dynamic(DynamicRef::<Local>::new(&7_u8))
        .expect_err("a different dynamic root type must be rejected");
    assert_eq!(error.expected(), std::any::TypeId::of::<VariantProbe>());
    assert_eq!(error.actual(), std::any::TypeId::of::<u8>());
}
