// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Typed keys for retrieving capability operation contracts.

use std::any::TypeId;
use std::marker::PhantomData;

use crate::identity::CapabilityId;

/// A stable capability ID paired with its expected Rust adapter type.
///
/// Constructing a key does not register a capability. The supplied ID has
/// already passed the authority checks performed by [`CapabilityId`].
pub struct CapabilityKey<A: 'static> {
    id: CapabilityId,
    adapter_type: TypeId,
    marker: PhantomData<fn() -> A>,
}

impl<A: 'static> CapabilityKey<A> {
    /// Creates an externally defined typed key from a validated capability ID.
    pub fn new(id: CapabilityId) -> Self {
        Self {
            id,
            adapter_type: TypeId::of::<A>(),
            marker: PhantomData,
        }
    }

    /// Creates a built-in typed key using this crate's reserved ID authority.
    pub(crate) fn new_core(id: &str) -> Self {
        let id = CapabilityId::new_core(id).expect("built-in capability IDs must use valid qubit.reflect names");
        Self::new(id)
    }

    /// Returns the stable capability identity.
    #[must_use]
    #[inline(always)]
    pub const fn id(&self) -> &CapabilityId {
        &self.id
    }

    /// Returns the process-local identity of the expected adapter contract.
    #[must_use]
    #[inline(always)]
    pub const fn adapter_type(&self) -> TypeId {
        self.adapter_type
    }
}

impl<A: 'static> Clone for CapabilityKey<A> {
    /// Clones the stable ID while retaining the same adapter contract.
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            adapter_type: self.adapter_type,
            marker: PhantomData,
        }
    }
}

impl<A: 'static> std::fmt::Debug for CapabilityKey<A> {
    /// Formats the stable ID and adapter contract identity.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CapabilityKey")
            .field("id", &self.id)
            .field("adapter_type", &self.adapter_type)
            .finish()
    }
}
