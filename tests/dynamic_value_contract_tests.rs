// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::TypeId;

use qubit_reflect::ReflectionContext;
use qubit_reflect::value::DynamicMut;
use qubit_reflect::value::DynamicOwned;
use qubit_reflect::value::DynamicRef;
use qubit_reflect::value::DynamicValue;
use qubit_reflect::value::DynamicValueKind;
use qubit_reflect::value::Local;

#[test]
fn public_dynamic_value_facade_supports_all_ownership_kinds() {
    let context = ReflectionContext::builder()
        .build()
        .expect("an empty explicit context is valid");

    let owned = DynamicValue::<Local>::Owned(DynamicOwned::new(11_u64));
    assert_eq!(owned.kind(), DynamicValueKind::Owned);
    assert_eq!(owned.type_id(), TypeId::of::<u64>());
    assert_eq!(owned.ty_in(&context).type_id(), TypeId::of::<u64>());
    assert_eq!(owned.as_ref().downcast_ref::<u64>(), Some(&11));

    let shared_value = 13_u32;
    let shared = DynamicValue::Ref(DynamicRef::<Local>::new(&shared_value));
    assert_eq!(shared.kind(), DynamicValueKind::Ref);
    assert_eq!(shared.type_id(), TypeId::of::<u32>());
    assert_eq!(shared.ty().type_id(), TypeId::of::<u32>());
    assert_eq!(shared.as_ref().downcast_ref::<u32>(), Some(&13));

    let mut mutable_value = 17_u16;
    let mutable = DynamicValue::Mut(DynamicMut::<Local>::new(&mut mutable_value));
    assert_eq!(mutable.kind(), DynamicValueKind::Mut);
    assert_eq!(mutable.type_id(), TypeId::of::<u16>());
    assert_eq!(mutable.as_ref().downcast_ref::<u16>(), Some(&17));
}
