// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::capability_target_id::CapabilityTargetId;
use crate::runtime::descriptor::TypeDescriptor;
use crate::runtime::descriptor::generic::TypeDefinitionDescriptor;

/// A capability target resolved in the context that performed the lookup.
#[derive(Clone, Copy)]
pub enum CapabilityTarget<'ctx> {
    /// A concrete reflected Rust type.
    Type(TypeDescriptor<'ctx>),
    /// A reflected source type definition.
    Definition(TypeDefinitionDescriptor<'ctx>),
}

impl CapabilityTarget<'_> {
    /// Returns the typed identity used to index this capability target.
    pub fn id(self) -> CapabilityTargetId {
        match self {
            Self::Type(descriptor) => CapabilityTargetId::Type(descriptor.type_identity()),
            Self::Definition(descriptor) => CapabilityTargetId::Definition(descriptor.id()),
        }
    }
}
