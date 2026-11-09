// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::TypeId;
use std::sync::Arc;

use super::capability_id::CapabilityIdentifier;
use super::capability_registration_origin::CapabilityRegistrationOrigin;

/// Immutable adapter registration retained by a target's capability set.
pub(crate) struct CapabilityRecord {
    /// Typed identifier used for capability lookup.
    pub(super) id: CapabilityIdentifier,
    /// Stable string form of the capability identifier.
    pub(super) id_str: &'static str,
    /// Runtime type identity of the registered adapter.
    pub(super) adapter_type: TypeId,
    /// Type-erased adapter retained by the capability registry.
    pub(super) adapter: Arc<dyn std::any::Any + Send + Sync>,
    /// Provenance recorded when the adapter was registered.
    pub(super) origin: CapabilityRegistrationOrigin,
}
