// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Deterministic immutable capability collections.

use std::any::TypeId;
use std::sync::OnceLock;

use crate::capability::CapabilityDescriptor;
use crate::capability::CapabilityKey;
use crate::identity::CapabilityId;

/// The machine-readable reason a capability set could not be formed.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CapabilityConflictKind {
    /// The same stable ID was declared more than once with one contract.
    DuplicateId,
    /// The same stable ID was assigned multiple Rust adapter contracts.
    AdapterTypeMismatch,
}

/// A conflict between two descriptors claiming one stable capability ID.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("conflicting reflection capability `{id}`: {kind:?}")]
pub struct CapabilityConflict {
    kind: CapabilityConflictKind,
    id: CapabilityId,
    first_adapter_type: TypeId,
    second_adapter_type: TypeId,
}

impl CapabilityConflict {
    /// Returns the machine-readable conflict class.
    #[must_use]
    #[inline(always)]
    pub const fn kind(&self) -> CapabilityConflictKind {
        self.kind
    }

    /// Returns the stable ID claimed by both descriptors.
    #[must_use]
    #[inline(always)]
    pub const fn id(&self) -> &CapabilityId {
        &self.id
    }

    /// Returns the first descriptor's process-local adapter contract identity.
    #[must_use]
    #[inline(always)]
    pub const fn first_adapter_type(&self) -> TypeId {
        self.first_adapter_type
    }

    /// Returns the second descriptor's process-local adapter contract identity.
    #[must_use]
    #[inline(always)]
    pub const fn second_adapter_type(&self) -> TypeId {
        self.second_adapter_type
    }
}

/// An immutable capability set sorted by stable capability ID.
///
/// # Examples
///
/// ```
/// use qubit_reflect::{ReflectRegistry, TypeDescriptor};
///
/// let registry = ReflectRegistry::initialize()?;
/// let capabilities = registry.capabilities(TypeDescriptor::of::<u32>());
/// assert!(capabilities.descriptors().windows(2).all(|pair| pair[0].id() < pair[1].id()));
/// # Ok::<(), qubit_reflect::RegistryError>(())
/// ```
#[derive(Clone, Debug)]
pub struct TypeCapabilities {
    descriptors: Box<[CapabilityDescriptor]>,
}

impl TypeCapabilities {
    /// Validates and sorts capability descriptors.
    ///
    /// Returns [`CapabilityConflict`] when an ID occurs more than once. A
    /// different adapter type is reported separately from an exact duplicate.
    pub fn try_new(mut descriptors: Vec<CapabilityDescriptor>) -> Result<Self, CapabilityConflict> {
        descriptors.sort_by(|left, right| {
            left.id()
                .cmp(right.id())
                .then_with(|| left.adapter_type().cmp(&right.adapter_type()))
        });
        for pair in descriptors.windows(2) {
            let [first, second] = pair else {
                continue;
            };
            if first.id() != second.id() {
                continue;
            }
            let kind = if first.adapter_type() == second.adapter_type() {
                CapabilityConflictKind::DuplicateId
            } else {
                CapabilityConflictKind::AdapterTypeMismatch
            };
            return Err(CapabilityConflict {
                kind,
                id: *first.id(),
                first_adapter_type: first.adapter_type(),
                second_adapter_type: second.adapter_type(),
            });
        }
        Ok(Self {
            descriptors: descriptors.into_boxed_slice(),
        })
    }

    /// Returns descriptors in stable capability-ID order.
    #[must_use]
    #[inline(always)]
    pub const fn descriptors(&self) -> &[CapabilityDescriptor] {
        &self.descriptors
    }

    /// Returns whether the set contains the key's exact ID and adapter
    /// contract.
    #[must_use]
    pub fn contains<A: 'static>(&self, key: CapabilityKey<A>) -> bool {
        self.find(key.id())
            .is_some_and(|descriptor| descriptor.adapter_type() == key.adapter_type())
    }

    /// Retrieves a capability adapter through its typed key.
    ///
    /// `None` means the ID is absent, the contract differs, or the descriptor
    /// represents a fact without an executable adapter.
    #[must_use]
    pub fn get<A: 'static>(&self, key: CapabilityKey<A>) -> Option<&A> {
        self.find(key.id())?.get(&key)
    }

    /// Finds a capability descriptor by its stable textual ID without
    /// allocating an owned identity.
    #[must_use]
    pub fn descriptor(&self, id: &str) -> Option<&CapabilityDescriptor> {
        let index = self
            .descriptors
            .binary_search_by(|descriptor| descriptor.id().as_str().cmp(id))
            .ok()?;
        self.descriptors.get(index)
    }

    /// Finds one descriptor by stable ID in the sorted collection.
    fn find(&self, id: &CapabilityId) -> Option<&CapabilityDescriptor> {
        let index = self
            .descriptors
            .binary_search_by(|descriptor| descriptor.id().cmp(id))
            .ok()?;
        self.descriptors.get(index)
    }
}

impl Default for TypeCapabilities {
    /// Creates an empty immutable capability set.
    fn default() -> Self {
        Self {
            descriptors: Box::default(),
        }
    }
}

/// Returns the shared empty set used by descriptors without registered
/// capabilities.
pub(crate) fn empty_capabilities() -> &'static TypeCapabilities {
    static EMPTY: OnceLock<TypeCapabilities> = OnceLock::new();
    EMPTY.get_or_init(TypeCapabilities::default)
}
