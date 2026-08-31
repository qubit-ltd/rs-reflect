// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0 (the "License");
//    you may not use this file except in compliance with the License.
// =============================================================================

//! Private materialized registry payload state.

use crate::identity::FragmentIdentity;
use crate::registry::fragment::FragmentPayload;

/// A built fragment retained until every cross-fragment check succeeds.
pub(crate) struct BuiltFragment {
    pub(crate) identity: FragmentIdentity,
    pub(crate) payload: FragmentPayload,
}
