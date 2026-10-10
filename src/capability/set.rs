// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Deterministic immutable capability collections.

use std::any::TypeId;
use std::sync::OnceLock;

use crate::capability::CapabilityAccessError;
use crate::capability::CapabilityDescriptor;
use crate::capability::CapabilityKey;
use crate::capability::CapabilityLookup;
use crate::identity::CapabilityId;

/// The machine-readable reason a capability set could not be formed.
///
/// # Examples
///
/// ```
/// use qubit_reflect::capability::CapabilityConflictKind;
///
/// let kind = CapabilityConflictKind::DuplicateId;
/// assert_eq!(kind, CapabilityConflictKind::DuplicateId);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CapabilityConflictKind {
    /// The same stable ID was declared more than once with one contract.
    DuplicateId,
    /// The same stable ID was assigned multiple Rust adapter contracts.
    AdapterTypeMismatch,
}

/// A conflict between two descriptors claiming one stable capability ID.
///
/// # Examples
///
/// ```
/// use qubit_reflect::capability::CapabilityConflict;
/// use qubit_reflect::capability::CapabilityDescriptor;
/// use qubit_reflect::capability::CapabilityKey;
/// use qubit_reflect::capability::TypeCapabilities;
/// use qubit_reflect::identity::CapabilityId;
///
/// let id = CapabilityId::new("example.conflict").expect("valid ID");
/// let first = CapabilityDescriptor::without_adapter(CapabilityKey::<u32>::new(id));
/// let second = CapabilityDescriptor::without_adapter(CapabilityKey::<u64>::new(id));
/// let error = TypeCapabilities::try_new(vec![first, second]).unwrap_err();
/// assert_eq!(error.kind(), qubit_reflect::capability::CapabilityConflictKind::AdapterTypeMismatch);
/// ```
#[must_use]
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("conflicting reflection capability `{id}`: {kind:?}")]
pub struct CapabilityConflict {
    /// Identifies whether the conflict is an exact duplicate or an adapter mismatch.
    kind: CapabilityConflictKind,
    /// Stable capability ID claimed by both descriptors.
    id: CapabilityId,
    /// Process-local adapter contract identity of the first descriptor.
    first_adapter_type: TypeId,
    /// Process-local adapter contract identity of the second descriptor.
    second_adapter_type: TypeId,
}

impl CapabilityConflict {
    /// Classifies two descriptors already known to claim the same capability
    /// ID while preserving their input contract order.
    ///
    /// # Parameters
    ///
    /// - `first`: The first descriptor claiming the ID.
    /// - `second`: The second descriptor claiming the same ID.
    ///
    /// # Returns
    ///
    /// Returns the classified conflict and preserves the input contract order.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if the descriptors have different IDs.
    pub(crate) fn from_same_id(first: &CapabilityDescriptor, second: &CapabilityDescriptor) -> Self {
        debug_assert_eq!(first.id(), second.id());
        let kind = if first.adapter_type() == second.adapter_type() {
            CapabilityConflictKind::DuplicateId
        } else {
            CapabilityConflictKind::AdapterTypeMismatch
        };
        Self {
            kind,
            id: *first.id(),
            first_adapter_type: first.adapter_type(),
            second_adapter_type: second.adapter_type(),
        }
    }

    /// Returns the machine-readable conflict class.
    ///
    /// # Returns
    ///
    /// Returns the conflict kind.
    #[must_use]
    #[inline]
    pub const fn kind(&self) -> CapabilityConflictKind {
        self.kind
    }

    /// Returns the stable ID claimed by both descriptors.
    ///
    /// # Returns
    ///
    /// Returns the shared capability ID.
    #[must_use]
    #[inline]
    pub const fn id(&self) -> &CapabilityId {
        &self.id
    }

    /// Returns the first descriptor's process-local adapter contract identity.
    ///
    /// # Returns
    ///
    /// Returns the first adapter contract's `TypeId`.
    #[must_use]
    #[inline]
    pub const fn first_adapter_type(&self) -> TypeId {
        self.first_adapter_type
    }

    /// Returns the second descriptor's process-local adapter contract identity.
    ///
    /// # Returns
    ///
    /// Returns the second adapter contract's `TypeId`.
    #[must_use]
    #[inline]
    pub const fn second_adapter_type(&self) -> TypeId {
        self.second_adapter_type
    }
}

/// An immutable capability set sorted by stable capability ID.
///
/// # Examples
///
/// ```no_run
/// use qubit_reflect::ReflectRegistry;
/// use qubit_reflect::TypeDescriptor;
///
/// let registry = ReflectRegistry::initialize()?;
/// let capabilities = registry.capabilities(TypeDescriptor::of::<u32>()).expect("valid capability declarations");
/// assert!(capabilities.descriptors().windows(2).all(|pair| pair[0].id() < pair[1].id()));
/// # Ok::<(), qubit_reflect::RegistryError>(())
/// ```
#[derive(Clone, Debug)]
pub struct TypeCapabilities {
    /// Descriptors sorted by stable capability ID.
    descriptors: Box<[CapabilityDescriptor]>,
}

impl TypeCapabilities {
    /// Validates and sorts capability descriptors.
    ///
    /// Returns [`CapabilityConflict`] when an ID occurs more than once. A
    /// different adapter type is reported separately from an exact duplicate.
    ///
    /// # Parameters
    ///
    /// - `descriptors`: The descriptors to validate and sort.
    ///
    /// # Returns
    ///
    /// Returns a capability set sorted by stable ID.
    ///
    /// # Errors
    ///
    /// Returns the conflicting ID and adapter contracts when an ID occurs
    /// more than once.
    pub fn try_new(mut descriptors: Vec<CapabilityDescriptor>) -> Result<Self, CapabilityConflict> {
        descriptors.sort_by(|left, right| left.id().cmp(right.id()));
        for pair in descriptors.windows(2) {
            let [first, second] = pair else {
                continue;
            };
            if first.id() != second.id() {
                continue;
            }
            return Err(CapabilityConflict::from_same_id(first, second));
        }
        Ok(Self {
            descriptors: descriptors.into_boxed_slice(),
        })
    }

    /// Returns descriptors in stable capability-ID order.
    ///
    /// # Returns
    ///
    /// Returns the set's descriptors sorted by stable ID.
    #[must_use]
    #[inline]
    pub const fn descriptors(&self) -> &[CapabilityDescriptor] {
        &self.descriptors
    }

    /// Returns whether the set contains the key's exact ID and adapter
    /// contract.
    ///
    /// # Type Parameters
    ///
    /// - `A`: The expected adapter contract type.
    ///
    /// # Parameters
    ///
    /// - `key`: The typed capability key to check.
    ///
    /// # Returns
    ///
    /// Returns `true` when a descriptor has the key's ID and adapter type.
    #[must_use]
    pub fn contains<A: 'static>(&self, key: CapabilityKey<A>) -> bool {
        self.find(key.id())
            .is_some_and(|descriptor| descriptor.adapter_type() == key.adapter_type())
    }

    /// Retrieves a capability adapter through its typed key.
    ///
    /// `Ok(None)` means the ID is absent. A fact without an executable adapter
    /// or a different adapter contract is returned as an error.
    ///
    /// # Type Parameters
    ///
    /// - `A`: The requested adapter contract type.
    ///
    /// # Parameters
    ///
    /// - `key`: The typed key used to retrieve the capability.
    ///
    /// # Returns
    ///
    /// Returns the typed adapter when its ID and contract match, or `None` when
    /// the ID is absent.
    ///
    /// `Err` distinguishes a fact-only capability from a mismatched adapter
    /// contract.
    ///
    /// # Errors
    ///
    /// Returns [`CapabilityAccessError::FactOnly`] when the ID names a
    /// non-executable fact and [`CapabilityAccessError::AdapterTypeMismatch`]
    /// when the ID is present with a different adapter contract.
    pub fn get<A: 'static>(&self, key: CapabilityKey<A>) -> Result<Option<&A>, CapabilityAccessError> {
        self.lookup(key).into_adapter()
    }

    /// Looks up a capability while preserving all four states: missing ID,
    /// fact-only descriptor, adapter contract mismatch, and executable adapter.
    ///
    /// # Type Parameters
    ///
    /// - `A`: The expected adapter contract type.
    ///
    /// # Parameters
    ///
    /// - `key`: The typed key used to classify the lookup.
    ///
    /// # Returns
    ///
    /// Returns the lookup state while preserving missing, fact-only, mismatch,
    /// and found outcomes.
    ///
    /// # Panics
    ///
    /// Panics if a descriptor records an adapter type that does not match its
    /// stored adapter value, violating the descriptor invariant.
    #[must_use]
    pub fn lookup<A: 'static>(&self, key: CapabilityKey<A>) -> CapabilityLookup<'_, A> {
        let Some(descriptor) = self.find(key.id()) else {
            return CapabilityLookup::Missing;
        };
        if descriptor.adapter_type() != key.adapter_type() {
            return CapabilityLookup::AdapterTypeMismatch {
                descriptor,
                expected: key.adapter_type(),
            };
        }
        if !descriptor.has_adapter() {
            return CapabilityLookup::FactOnly(descriptor);
        }
        CapabilityLookup::Found(descriptor.get(&key).expect("declared adapter contract must downcast"))
    }

    /// Finds a capability descriptor by its stable textual ID without
    /// allocating an owned identity.
    ///
    /// # Parameters
    ///
    /// - `id`: The stable textual capability ID.
    ///
    /// # Returns
    ///
    /// Returns the matching descriptor, or `None` if the ID is absent.
    pub fn descriptor(&self, id: &str) -> Option<&CapabilityDescriptor> {
        let index = self
            .descriptors
            .binary_search_by(|descriptor| descriptor.id().as_str().cmp(id))
            .ok()?;
        self.descriptors.get(index)
    }

    /// Finds one descriptor by stable ID in the sorted collection.
    ///
    /// # Parameters
    ///
    /// - `id`: The validated capability ID to find.
    ///
    /// # Returns
    ///
    /// Returns the matching descriptor, or `None` if absent.
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
    ///
    /// # Returns
    ///
    /// Returns a capability set with no descriptors.
    fn default() -> Self {
        Self {
            descriptors: Box::default(),
        }
    }
}

/// Returns the shared empty set used by descriptors without registered
/// capabilities.
///
/// # Returns
///
/// Returns the process-wide empty capability set.
#[must_use]
pub(crate) fn empty_capabilities() -> &'static TypeCapabilities {
    static EMPTY: OnceLock<TypeCapabilities> = OnceLock::new();
    EMPTY.get_or_init(TypeCapabilities::default)
}

/// A lazily initialized capability set or its structural conflict.
///
/// The result contains the shared set after successful initialization, or the
/// conflict that prevented construction.
///
/// # Examples
///
/// ```no_run
/// use qubit_reflect::capability::TypeCapabilitiesResult;
/// use qubit_reflect::ReflectRegistry;
/// use qubit_reflect::TypeDescriptor;
///
/// let registry = ReflectRegistry::initialize()?;
/// let result: TypeCapabilitiesResult = registry.capabilities(TypeDescriptor::of::<u32>());
/// assert!(result.is_ok());
/// # Ok::<(), qubit_reflect::RegistryError>(())
/// ```
pub type TypeCapabilitiesResult = Result<&'static TypeCapabilities, CapabilityConflict>;
