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
use crate::value::DynamicOwned;
use crate::value::Local;
use crate::value::ReflectedRef;

fn declaring_type() -> &'static TypeDescriptor {
    TypeDescriptor::of::<u8>()
}

fn active_test(value: ReflectedRef<'_>) -> Result<bool, TypeMismatch> {
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

    let named = NamedConstructionInput::<Local>::new(std::iter::empty::<(&'static str, DynamicOwned<Local>)>());
    let tuple = TupleConstructionInput::<Local>::new(std::iter::empty::<DynamicOwned<Local>>());
    assert!(variant.construct_struct(named).is_err());
    assert!(variant.construct_tuple(tuple).is_err());
    assert!(variant.construct_unit().is_err());
}
