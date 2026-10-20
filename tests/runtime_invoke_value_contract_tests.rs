// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::invoke::InvocationValue;
use qubit_reflect::value::DynamicOwned;
use qubit_reflect::value::DynamicValue;
use qubit_reflect::value::DynamicValueKind;
use qubit_reflect::value::Local;

#[test]
fn invocation_value_is_the_public_dynamic_value_type() {
    let value: InvocationValue<'_, Local> = DynamicValue::Owned(DynamicOwned::new(23_u32));
    assert_eq!(value.kind(), DynamicValueKind::Owned);
    let recovered = match value.into_owned::<u32>() {
        Ok(value) => value,
        Err(_) => panic!("owned value matches"),
    };
    assert_eq!(recovered, 23);
}

#[test]
fn dynamic_value_recovery_errors_preserve_the_original_tagged_value() {
    let owned = DynamicValue::<Local>::Owned(DynamicOwned::new(String::from("owned")));
    let owned = owned
        .into_owned::<u64>()
        .expect_err("owned value with a different type is retained");
    assert_eq!(owned.kind(), DynamicValueKind::Owned);
    let recovered = match owned.into_owned::<String>() {
        Ok(value) => value,
        Err(_) => panic!("original type is retained"),
    };
    assert_eq!(recovered, "owned");

    let shared_value = String::from("shared");
    let shared = DynamicValue::Ref(qubit_reflect::value::DynamicRef::<Local>::new(&shared_value));
    let shared = shared
        .into_ref::<str>()
        .expect_err("shared value with a different type is retained");
    assert_eq!(shared.kind(), DynamicValueKind::Ref);
    assert_eq!(
        shared.as_ref().downcast_ref::<String>().map(String::as_str),
        Some("shared")
    );

    let mut mutable_value = String::from("mutable");
    let mutable = DynamicValue::Mut(qubit_reflect::value::DynamicMut::<Local>::new(&mut mutable_value));
    let mutable = mutable
        .into_mut::<u64>()
        .expect_err("mutable value with a different type is retained");
    assert_eq!(mutable.kind(), DynamicValueKind::Mut);
    mutable
        .as_ref()
        .downcast_ref::<String>()
        .expect("original mutable value remains available");
}
