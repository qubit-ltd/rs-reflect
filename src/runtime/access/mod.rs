// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![allow(unsafe_code)]

mod field_adapter;

pub(crate) use field_adapter::DynamicFieldAdapter;
pub(crate) use field_adapter::DynamicFieldMode;
#[cfg(test)]
pub(crate) use field_adapter::DynamicOnlyFieldAdapter;
pub(crate) use field_adapter::FieldAdapter;
pub use field_adapter::TypedFieldAdapter;
