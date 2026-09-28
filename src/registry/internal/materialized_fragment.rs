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
    /// Stable source identity declared by the static registration record.
    pub(crate) identity: FragmentIdentity,
    /// Payload category declared by the static registration record.
    pub(crate) declared_kind: FragmentKind,
    /// Process-local target declared by the static registration record.
    pub(crate) declared_target: RuntimeIdentity,
    /// Materialized payload produced during registry initialization.
    pub(crate) payload: FragmentPayload,
}
