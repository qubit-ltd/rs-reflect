// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Reflection error types.

mod id_error;
mod registry_error;
mod type_mismatch;

pub use id_error::IdError;
pub use registry_error::RegistryError;
pub use registry_error::RegistryErrorKind;
pub use type_mismatch::TypeMismatch;
