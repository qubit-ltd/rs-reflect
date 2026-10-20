// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::runtime::descriptor::generic::TypeDefinitionId;
use crate::runtime::identity::TypeIdentity;

/// Identifies the reflection entity that owns a capability.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum CapabilityTargetId {
    /// A concrete registered Rust type.
    Type(TypeIdentity),
    /// A source type definition identified by its process-local marker.
    Definition(TypeDefinitionId),
}
