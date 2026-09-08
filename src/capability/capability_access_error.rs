// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::TypeId;

use crate::capability::CapabilityConflict;
use crate::identity::CapabilityId;

/// An error returned when a typed capability exists but cannot be executed.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum CapabilityAccessError {
    /// The intrinsic capability declarations conflict.
    #[error("intrinsic capability conflict: {0}")]
    IntrinsicConflict(#[source] CapabilityConflict),
    /// The requested capability is a fact without an executable adapter.
    #[error("capability {id} has no adapter for contract {adapter_type:?}")]
    FactOnly {
        /// The stable ID of the fact-only capability.
        id: CapabilityId,
        /// The adapter contract declared by the capability.
        adapter_type: TypeId,
    },
    /// The requested adapter contract differs from the declared contract.
    #[error(
        "capability {id} adapter mismatch: expected {expected:?}, actual {actual:?}"
    )]
    AdapterTypeMismatch {
        /// The stable ID of the capability.
        id: CapabilityId,
        /// The adapter contract requested by the caller.
        expected: TypeId,
        /// The adapter contract declared by the capability.
        actual: TypeId,
    },
}
