// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! APIs for reflected field and variant access.

mod error;
mod field_access_operation;
mod field_access_policy;
pub(crate) mod field_adapter;
mod field_identity;
mod field_set_recovery;
mod field_visibility;
mod variant_adapter;

pub use error::FieldAccessError;
pub use field_access_operation::FieldAccessOperation;
pub use field_access_policy::FieldAccessPolicy;
pub use field_adapter::FieldGetAdapter;
pub use field_adapter::FieldGetMutAdapter;
pub use field_adapter::FieldSetAdapter;
#[doc(hidden)]
pub use field_adapter::FieldSetPreflightAdapter;
pub use field_identity::FieldIdentity;
pub use field_set_recovery::FieldSetFailure;
pub use field_set_recovery::FieldSetRecovery;
pub use field_visibility::FieldVisibility;
pub use variant_adapter::VariantActiveAdapter;
