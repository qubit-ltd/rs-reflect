// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Verifies opt-in reflection implementations for external value types.

#[cfg(feature = "ecosystem-types")]
use bigdecimal::BigDecimal;
#[cfg(feature = "ecosystem-types")]
use chrono::DateTime;
#[cfg(feature = "ecosystem-types")]
use chrono::NaiveDate;
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
#[cfg(feature = "ecosystem-types")]
use uuid::Uuid;

#[cfg(feature = "ecosystem-types")]
#[test]
fn test_ecosystem_types_expose_opaque_reflection_roots() {
    for descriptor in [
        TypeDescriptor::of::<BigDecimal>(),
        TypeDescriptor::of::<DateTime<Utc>>(),
        TypeDescriptor::of::<NaiveDate>(),
        TypeDescriptor::of::<NaiveTime>(),
        TypeDescriptor::of::<Uuid>(),
    ] {
        assert!(descriptor.as_opaque().is_some());
    }
}

#[cfg(feature = "qubit-types")]
#[test]
fn test_qubit_types_expose_opaque_reflection_roots() {
    for descriptor in [TypeDescriptor::of::<Id>(), TypeDescriptor::of::<DataType>()] {
        assert!(descriptor.as_opaque().is_some());
    }
}
