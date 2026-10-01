// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Typed keys for retrieving capability operation contracts.

use std::any::TypeId;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use std::marker::PhantomData;

use crate::identity::CapabilityId;

/// A stable capability ID paired with its expected Rust adapter type.
///
/// Constructing a key does not register a capability. The supplied ID has
/// already passed the authority checks performed by [`CapabilityId`].
///
/// # Type Parameters
///
/// - `A`: The adapter contract type associated with this key.
///
/// # Examples
///
/// ```
/// use std::any::TypeId;
/// use qubit_reflect::capability::CapabilityKey;
/// use qubit_reflect::identity::CapabilityId;
///
/// let id = CapabilityId::new("example.adapter").expect("valid capability ID");
/// let key = CapabilityKey::<u32>::new(id);
/// assert_eq!(key.adapter_type(), TypeId::of::<u32>());
/// ```
pub struct CapabilityKey<A: 'static> {
    /// Stable ID of the capability described by this key.
    id: CapabilityId,
    /// Process-local identity of `A`, checked before adapter downcasting.
    adapter_type: TypeId,
    /// Keeps `A` invariant so lifetime subtyping cannot disagree with its
    /// `TypeId`.
    marker: PhantomData<fn(A) -> A>,
}

impl<A: 'static> CapabilityKey<A> {
    /// Creates an externally defined typed key from a validated capability ID.
    ///
    /// # Type Parameters
    ///
    /// - `A`: The adapter contract type associated with the key.
    ///
    /// # Parameters
    ///
    /// - `id`: The validated stable capability ID.
    ///
    /// # Returns
    ///
    /// Returns a key associated with `id` and the type identity of `A`.
    #[must_use]
    #[inline]
    pub fn new(id: CapabilityId) -> Self {
        Self {
            id,
            adapter_type: TypeId::of::<A>(),
            marker: PhantomData,
        }
    }

    /// Creates a built-in typed key using this crate's reserved ID authority.
    ///
    /// # Parameters
    ///
    /// - `id`: The reserved built-in capability ID.
    ///
    /// # Returns
    ///
    /// Returns a typed key for the reserved ID.
    ///
    /// # Panics
    ///
    /// Panics if `id` is not a valid reserved `qubit.reflect` capability ID.
    #[inline]
    pub(crate) fn new_core(id: &'static str) -> Self {
        let id = CapabilityId::new_core(id).expect("built-in capability IDs must use valid qubit.reflect names");
        Self::new(id)
    }

    /// Returns the stable capability identity.
    ///
    /// # Returns
    ///
    /// Returns the key's stable capability ID.
    #[must_use]
    #[inline]
    pub const fn id(&self) -> &CapabilityId {
        &self.id
    }

    /// Returns the process-local identity of the expected adapter contract.
    ///
    /// # Returns
    ///
    /// Returns the adapter contract's process-local `TypeId`.
    #[must_use]
    #[inline]
    pub const fn adapter_type(&self) -> TypeId {
        self.adapter_type
    }
}

impl<A: 'static> Clone for CapabilityKey<A> {
    /// Copies the static ID while retaining the same adapter contract.
    ///
    /// # Returns
    ///
    /// Returns a copy of this typed key.
    fn clone(&self) -> Self {
        *self
    }
}

impl<A: 'static> Copy for CapabilityKey<A> {}

impl<A: 'static> Debug for CapabilityKey<A> {
    /// Formats the stable ID and adapter contract identity.
    ///
    /// # Parameters
    ///
    /// - `formatter`: The destination formatter.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after formatting the key.
    ///
    /// # Errors
    ///
    /// Returns a formatting error if writing to `formatter` fails.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        formatter
            .debug_struct("CapabilityKey")
            .field("id", &self.id)
            .field("adapter_type", &self.adapter_type)
            .finish()
    }
}
