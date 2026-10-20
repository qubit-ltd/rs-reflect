// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Verifies opt-in reflection implementations for external value types.

#[cfg(any(feature = "ecosystem-types", feature = "qubit-types"))]
use std::any::TypeId;

#[cfg(feature = "ecosystem-types")]
use bigdecimal::BigDecimal;
#[cfg(feature = "ecosystem-types")]
use chrono::DateTime;
#[cfg(feature = "ecosystem-types")]
use chrono::NaiveDate;
#[cfg(feature = "ecosystem-types")]
use chrono::NaiveDateTime;
#[cfg(feature = "ecosystem-types")]
use chrono::NaiveTime;
#[cfg(feature = "ecosystem-types")]
use chrono::Utc;
#[cfg(feature = "qubit-types")]
use qubit_datatype::DataType;
#[cfg(feature = "qubit-types")]
use qubit_id::Id;
#[cfg(any(feature = "ecosystem-types", feature = "qubit-types"))]
use qubit_reflect::TypeDescriptor;
#[cfg(any(feature = "ecosystem-types", feature = "qubit-types"))]
use qubit_reflect::descriptor::TypeKind;
#[cfg(feature = "ecosystem-types")]
use uuid::Uuid;

#[cfg(feature = "ecosystem-types")]
#[test]
fn test_ecosystem_types_expose_opaque_reflection_roots() {
    assert_opaque::<BigDecimal>();
    assert_opaque::<DateTime<Utc>>();
    assert_opaque::<NaiveDate>();
    assert_opaque::<NaiveDateTime>();
    assert_opaque::<NaiveTime>();
    assert_opaque::<Uuid>();
}

#[cfg(feature = "qubit-types")]
#[test]
fn test_qubit_types_expose_opaque_reflection_roots() {
    assert_opaque::<Id>();
    assert_opaque::<DataType>();
}

#[cfg(any(feature = "ecosystem-types", feature = "qubit-types"))]
fn assert_opaque<T: qubit_reflect::descriptor::Reflect + ?Sized>() {
    let descriptor = TypeDescriptor::of::<T>();

    assert_eq!(descriptor.type_id(), TypeId::of::<T>());
    match descriptor.kind() {
        TypeKind::Opaque(opaque) => assert_eq!(opaque.descriptor().type_id(), TypeId::of::<T>()),
        _kind => panic!(
            "expected an opaque descriptor for {}, got another kind",
            descriptor.rust_type_name()
        ),
    }
}
