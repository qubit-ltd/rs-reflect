// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Reflection roots for optional external value types.

use crate::runtime::descriptor::reflect::Reflect;
use crate::runtime::descriptor::reflect::TypeRegistration;
use crate::runtime::descriptor::type_kind::TypeKindMetadata;

static OPAQUE_KIND: TypeKindMetadata = TypeKindMetadata::Opaque;

macro_rules! impl_opaque_reflect {
    ($($type:ty),+ $(,)?) => {
        $(
            impl Reflect for $type {
                fn __reflect_registration() -> &'static TypeRegistration<Self> {
                    TypeRegistration::get_or_init_with(|| {
                        TypeRegistration::with_shape(&OPAQUE_KIND, None, &[])
                    })
                }
            }
            crate::__private::codegen_v4::registration::submit_type!($type);
        )+
    };
}

#[cfg(feature = "ecosystem-types")]
impl_opaque_reflect!(
    bigdecimal::BigDecimal,
    chrono::DateTime<chrono::Utc>,
    chrono::NaiveDate,
    chrono::NaiveDateTime,
    chrono::NaiveTime,
    uuid::Uuid,
);

#[cfg(feature = "qubit-types")]
impl_opaque_reflect!(qubit_id::Id, qubit_datatype::DataType);
