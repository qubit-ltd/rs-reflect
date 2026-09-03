// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! One prepared registry fact used by benchmark aggregation.

use std::any::TypeId;

use crate::capability::CapabilityDescriptor;
use crate::identity::FragmentIdentity;

/// One prepared benchmark fact kept outside the measured aggregation loop.
pub(super) struct FactRow {
    /// Stable source identity used by registry diagnostics.
    pub(super) identity: FragmentIdentity,
    /// Concrete type receiving the benchmark capability.
    pub(super) target_type_id: TypeId,
    /// Adapter-free capability descriptor being registered.
    pub(super) descriptor: CapabilityDescriptor,
}
