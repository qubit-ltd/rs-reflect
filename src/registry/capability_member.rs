// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Diagnostic results for capability lookups on registry members.

use crate::capability::CapabilityLookup;
use crate::capability::CapabilityOrigin;
use crate::identity::FragmentIdentity;

/// One registered member carrying a capability with its typed lookup result.
///
/// The value retains the capability state, its semantic origin, and the
/// fragment associated with the effective fact. It is created by
/// [`ReflectRegistry`](super::ReflectRegistry) queries and cannot be assembled
/// directly by callers.
///
/// # Type Parameters
///
/// - `T`: The static type or generic-definition descriptor carried by the
///   member.
/// - `A`: The typed adapter contract requested from the registry.
///
/// # Examples
///
/// ```
/// use qubit_reflect::capability::CapabilityDescriptor;
/// use qubit_reflect::capability::CapabilityKey;
/// use qubit_reflect::capability::CapabilityLookup;
/// use qubit_reflect::identity::CapabilityId;
/// use qubit_reflect::identity::FragmentIdentity;
/// use qubit_reflect::registry::RegistrySnapshotBuilder;
/// use qubit_reflect::TypeDescriptor;
///
/// let key = CapabilityKey::<u32>::new(CapabilityId::new("example.limit")?);
/// let mut builder = RegistrySnapshotBuilder::new();
/// builder.add_type(
///     TypeDescriptor::of::<u32>(),
///     FragmentIdentity::new("example", "settings", 1, 1, "type", 1),
/// );
/// builder.add_type_capabilities(
///     TypeDescriptor::of::<u32>(),
///     vec![CapabilityDescriptor::with_adapter(key, 7)],
///     FragmentIdentity::new("example", "settings", 1, 1, "capability", 1),
/// );
/// let registry = builder.build()?;
/// let member = registry.type_capability_members(key).next().expect("registered member");
/// assert!(matches!(member.lookup(), CapabilityLookup::Found(value) if **value == 7));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[must_use]
#[derive(Debug)]
pub struct CapabilityMember<'registry, T, A: 'static> {
    target: T,
    lookup: CapabilityLookup<'registry, A>,
    origin: CapabilityOrigin,
    source: &'registry FragmentIdentity,
}

impl<'registry, T: Copy, A: 'static> CapabilityMember<'registry, T, A> {
    /// Creates a lookup result from one validated registry entry.
    ///
    /// # Parameters
    ///
    /// - `target`: The member descriptor returned by the registry.
    /// - `lookup`: The typed result for the requested capability ID.
    /// - `origin`: The semantic origin of the effective capability.
    /// - `source`: The fragment associated with the effective capability.
    ///
    /// # Returns
    ///
    /// Returns an immutable member result. This constructor is restricted to
    /// the registry implementation.
    pub(crate) const fn new(
        target: T,
        lookup: CapabilityLookup<'registry, A>,
        origin: CapabilityOrigin,
        source: &'registry FragmentIdentity,
    ) -> Self {
        Self {
            target,
            lookup,
            origin,
            source,
        }
    }

    /// Returns the type or generic definition that carries the capability.
    ///
    /// # Returns
    ///
    /// Returns the static descriptor selected by the registry iterator.
    #[must_use]
    #[inline]
    pub const fn target(&self) -> T {
        self.target
    }

    /// Returns the typed capability state for this member.
    ///
    /// The result is always `Found`, `FactOnly`, or `AdapterTypeMismatch`;
    /// members without the requested stable capability ID are not yielded.
    ///
    /// # Returns
    ///
    /// Returns the lookup state while borrowing its adapter or descriptor from
    /// the registry snapshot.
    #[must_use]
    #[inline]
    pub const fn lookup(&self) -> &CapabilityLookup<'registry, A> {
        &self.lookup
    }

    /// Returns whether the fact was declared intrinsically or registered.
    ///
    /// # Returns
    ///
    /// Returns the origin retained when the registry snapshot was built.
    #[must_use]
    #[inline]
    pub const fn origin(&self) -> &CapabilityOrigin {
        &self.origin
    }

    /// Returns the fragment associated with this effective capability fact.
    ///
    /// For an intrinsic fact on a registered type or definition, this is the
    /// member's declaration fragment. For an intrinsic capability-only target,
    /// it is the earliest fragment that caused that target to be inspected.
    ///
    /// # Returns
    ///
    /// Returns the fragment identity borrowed from the registry snapshot.
    #[must_use]
    #[inline]
    pub const fn source(&self) -> &'registry FragmentIdentity {
        self.source
    }
}
