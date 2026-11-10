// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Shared implementation helpers used by construction facade modules.

/// Copies the expected and actual type identities into the public error type.
///
/// # Parameters
///
/// - `inner`: Runtime mismatch containing the expected and actual type
///   identities.
pub(super) fn public_mismatch(inner: &crate::runtime::value::TypeMismatch) -> crate::error::TypeMismatch {
    inner.clone()
}
