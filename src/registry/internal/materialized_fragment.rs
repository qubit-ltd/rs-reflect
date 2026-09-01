// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Private post-materialization registry fragment state.

use crate::identity::FragmentIdentity;
use crate::registry::fragment::FragmentKind;
use crate::registry::fragment::FragmentPayload;
use crate::registry::fragment::RuntimeIdentity;

/// A materialized payload plus the declarations from its static record.
pub(crate) struct MaterializedFragment {
    pub(crate) identity: FragmentIdentity,
    pub(crate) declared_kind: FragmentKind,
    pub(crate) declared_target: RuntimeIdentity,
    pub(crate) payload: FragmentPayload,
}
