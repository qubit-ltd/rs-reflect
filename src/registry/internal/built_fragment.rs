// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Private materialized registry payload state.

use crate::identity::FragmentIdentity;
use crate::registry::fragment::FragmentPayload;

/// A built fragment retained until every cross-fragment check succeeds.
pub(crate) struct BuiltFragment {
    /// Stable source identity used to report aggregation errors.
    pub(crate) identity: FragmentIdentity,
    /// Materialized registration facts awaiting cross-fragment validation.
    pub(crate) payload: FragmentPayload,
}
