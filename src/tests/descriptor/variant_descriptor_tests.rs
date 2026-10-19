// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0 (the "License");
//    you may not use this file except in compliance with the License.
// =============================================================================

use crate::runtime::descriptor::TypeDescriptor;
use crate::runtime::descriptor::variant::VariantShape;
use crate::runtime::value::{DynamicRef, Local};

#[cfg(feature = "derive")]
#[crate::Reflect]
enum VariantProbe {
    Ready,
    Waiting,
}

#[test]
#[cfg(feature = "derive")]
fn test_variant_descriptor_exposes_source_facts_and_activity() {
    let descriptor = TypeDescriptor::of::<VariantProbe>();
    let ready = descriptor.variant("Ready").expect("Ready variant exists");
    let waiting = descriptor.variant("Waiting").expect("Waiting variant exists");

    assert_eq!(ready.index(), 0);
    assert_eq!(ready.name(), "Ready");
    assert_eq!(ready.shape(), VariantShape::Unit);
    assert!(ready.fields().is_empty());
    assert!(ready.field("missing").is_err());
    assert!(ready.is_active(&VariantProbe::Ready).unwrap());
    assert!(!ready.is_active(&VariantProbe::Waiting).unwrap());
    assert!(waiting.is_active_dynamic(DynamicRef::<Local>::new(&VariantProbe::Waiting)).unwrap());
    assert!(!waiting.is_active_dynamic(DynamicRef::<Local>::new(&VariantProbe::Ready)).unwrap());
}

#[test]
#[cfg(feature = "derive")]
fn test_variant_activity_rejects_a_different_owner_type() {
    let ready = TypeDescriptor::of::<VariantProbe>()
        .variant("Ready")
        .expect("Ready variant exists");
    let error = ready
        .is_active(&7_u8)
        .expect_err("the activity adapter must reject a different owner type");

    assert_eq!(error.expected().type_id, std::any::TypeId::of::<VariantProbe>());
    assert_eq!(error.actual().type_id, std::any::TypeId::of::<u8>());
}
