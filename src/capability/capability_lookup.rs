// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Strict outcomes for typed capability lookups.

use std::any::TypeId;

use crate::capability::CapabilityAccessError;
use crate::capability::CapabilityDescriptor;

/// The result of looking up one typed capability contract by stable ID.
///
/// The four variants deliberately distinguish an absent ID, a fact without an
/// executable adapter, a contract mismatch, and an adapter that can run.
#[derive(Debug)]
pub enum CapabilityLookup<'a, A: 'static> {
    /// No descriptor carries the requested stable capability ID.
    Missing,
    /// The ID and adapter contract match, but no executable adapter exists.
    FactOnly(&'a CapabilityDescriptor),
    /// The stable ID exists, but its declared adapter contract differs.
    AdapterTypeMismatch {
        /// The descriptor carrying the requested stable ID.
        descriptor: &'a CapabilityDescriptor,
        /// The adapter type required by the typed lookup key.
        expected: TypeId,
    },
    /// The ID, adapter contract, and executable adapter all match.
    Found(&'a A),
}

impl<'a, A: 'static> CapabilityLookup<'a, A> {
    /// Returns the executable adapter, degrading every diagnostic state to
    /// absence.
    pub const fn into_adapter(self) -> Result<Option<&'a A>, CapabilityAccessError> {
        match self {
            Self::Found(value) => Ok(Some(value)),
            Self::Missing => Ok(None),
            Self::FactOnly(descriptor) => Err(CapabilityAccessError::FactOnly {
                id: *descriptor.id(),
                adapter_type: descriptor.adapter_type(),
            }),
            Self::AdapterTypeMismatch { descriptor, expected } => Err(CapabilityAccessError::AdapterTypeMismatch {
                id: *descriptor.id(),
                expected,
                actual: descriptor.adapter_type(),
            }),
        }
    }
}
