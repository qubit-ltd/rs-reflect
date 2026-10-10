// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Frozen registry snapshot and public deterministic lookup views.

use std::any::TypeId;
use std::sync::OnceLock;

use crate::capability::CapabilityAccessError;
use crate::capability::CapabilityConflict;
use crate::capability::CapabilityDescriptor;
use crate::capability::CapabilityKey;
use crate::capability::CapabilityLookup;
use crate::capability::CapabilityOrigin;
use crate::capability::TypeCapabilities;
use crate::descriptor::ImplDefinitionDescriptor;
use crate::descriptor::ImplDescriptor;
use crate::descriptor::TraitDefinitionDescriptor;
use crate::descriptor::TraitId;
use crate::descriptor::TypeDefinitionDescriptor;
use crate::descriptor::TypeDefinitionId;
use crate::descriptor::TypeDescriptor;
use crate::error::RegistryError;
use crate::expression::TypeExpression;
use crate::identity::FragmentIdentity;
use crate::registry::EffectiveTypeView;
use crate::registry::ImplDefinitionCandidates;
use crate::registry::TraitCandidates;
use crate::registry::TypeCandidates;
use crate::registry::TypeDefinitionCandidates;
use crate::registry::capability_member::CapabilityMember;
use crate::registry::fragment::CapabilityTarget;
use crate::registry::indexes::RegistryIndexes;
use crate::registry::registry_builder::build_inventory_registry;
use crate::registry::registry_builder::initialize_cached;

/// An immutable snapshot of validated reflection fragments.
///
/// [`Self::initialize`] resolves linked inventory once for the process;
/// [`super::RegistrySnapshotBuilder`] constructs independent snapshots from
/// explicit facts. Pass the selected snapshot to both method lookup and
/// invocation. Receiver capabilities never fall back to another registry.
/// Invocation outputs and futures do not borrow this snapshot.
///
/// # Examples
///
/// ```
/// use std::any::TypeId;
/// use qubit_reflect::registry::RegistrySnapshotBuilder;
///
/// let registry = RegistrySnapshotBuilder::new().build()?;
/// assert!(registry.types().is_empty());
/// assert!(registry.get(TypeId::of::<u8>()).is_none());
/// # Ok::<(), qubit_reflect::error::RegistryError>(())
/// ```
#[derive(Debug)]
pub struct ReflectRegistry {
    /// Registered concrete roots in stable fragment order.
    pub(super) types: Box<[&'static TypeDescriptor]>,
    /// Registered generic declarations in stable fragment order.
    pub(super) definitions: Box<[&'static TypeDefinitionDescriptor]>,
    /// Registered impl declarations in stable source-fragment order.
    pub(super) impl_definitions: Box<[&'static ImplDefinitionDescriptor]>,
    /// Immutable indexes built from the validated fragments.
    pub(super) indexes: RegistryIndexes,
    /// Stable empty effective-method view used for unregistered targets.
    pub(super) empty_effective_view: EffectiveTypeView,
    /// Stable empty capability set used for registered roots without facts.
    pub(super) empty_capabilities: TypeCapabilities,
}

impl ReflectRegistry {
    /// Initializes and returns the process-wide immutable registry snapshot.
    ///
    /// All linked fragments are sorted, materialized, and validated before a
    /// snapshot is published. Both success and [`RegistryError`] are cached;
    /// concurrent callers therefore observe the same immutable result.
    ///
    /// # Returns
    ///
    /// Returns the process-wide immutable registry snapshot.
    ///
    /// # Errors
    ///
    /// Returns the cached fragment validation error if linked registrations
    /// conflict or cannot be materialized consistently.
    pub fn initialize() -> Result<&'static Self, RegistryError> {
        static REGISTRY: OnceLock<Result<ReflectRegistry, RegistryError>> = OnceLock::new();
        initialize_cached(&REGISTRY, build_inventory_registry)
    }

    /// Looks up one exact concrete registered type.
    ///
    /// `None` means no linked static fragment registered `type_id`.
    ///
    /// # Parameters
    ///
    /// - `type_id`: Exact process-local identity to find.
    ///
    /// # Returns
    ///
    /// Returns the registered root, or `None` when absent.
    #[must_use]
    #[inline]
    pub fn get(&self, type_id: TypeId) -> Option<&'static TypeDescriptor> {
        self.indexes.types_by_id.get(&type_id).copied()
    }

    /// Finds every descriptor with the diagnostic Rust type name `name`.
    ///
    /// The returned view is empty when no descriptor matches and preserves the
    /// registry's stable fragment order when the name is ambiguous.
    ///
    /// # Parameters
    ///
    /// - `name`: Compiler-provided diagnostic Rust type name.
    ///
    /// # Returns
    ///
    /// Returns every matching root in stable order.
    #[must_use]
    pub fn find_by_type_name(&self, name: &str) -> TypeCandidates<'_> {
        TypeCandidates::new(self.indexes.types_by_type_name.get(name).map_or(&[], Box::as_ref))
    }

    /// Finds every descriptor with the reflection query name `name`.
    ///
    /// The returned view is empty when no descriptor matches and preserves the
    /// registry's stable fragment order when the name is ambiguous.
    ///
    /// # Parameters
    ///
    /// - `name`: Reflection query name.
    ///
    /// # Returns
    ///
    /// Returns every matching root in stable order.
    #[must_use]
    pub fn find_by_query_name(&self, name: &str) -> TypeCandidates<'_> {
        TypeCandidates::new(self.indexes.types_by_query_name.get(name).map_or(&[], Box::as_ref))
    }

    /// Enumerates all statically registered roots in stable fragment order.
    ///
    /// # Returns
    ///
    /// Returns every registered concrete root.
    #[must_use]
    #[inline]
    pub fn types(&self) -> &[&'static TypeDescriptor] {
        &self.types
    }

    /// Enumerates all registered generic type declarations in fragment order.
    ///
    /// # Returns
    ///
    /// Returns every registered generic declaration.
    #[must_use]
    #[inline]
    pub fn definitions(&self) -> &[&'static TypeDefinitionDescriptor] {
        &self.definitions
    }

    /// Looks up one generic declaration by its process-local identity.
    ///
    /// # Parameters
    ///
    /// - `id`: Process-local declaration identity.
    ///
    /// # Returns
    ///
    /// Returns the declaration, or `None` when it is absent.
    #[must_use]
    #[inline]
    pub fn definition(&self, id: TypeDefinitionId) -> Option<&'static TypeDefinitionDescriptor> {
        self.indexes.definitions_by_id.get(&id).copied()
    }

    /// Finds generic declarations with the exact Rust source path.
    ///
    /// # Parameters
    ///
    /// - `path`: Fully qualified Rust source path.
    ///
    /// # Returns
    ///
    /// Returns all exact path matches in stable order.
    #[must_use]
    pub fn find_definitions_by_rust_path(&self, path: &str) -> TypeDefinitionCandidates<'_> {
        TypeDefinitionCandidates::new(self.indexes.definitions_by_rust_path.get(path).map_or(&[], Box::as_ref))
    }

    /// Finds generic declarations with the exact reflection query name.
    ///
    /// # Parameters
    ///
    /// - `name`: Reflection query name.
    ///
    /// # Returns
    ///
    /// Returns all exact name matches in stable order.
    #[must_use]
    pub fn find_definitions_by_query_name(&self, name: &str) -> TypeDefinitionCandidates<'_> {
        TypeDefinitionCandidates::new(
            self.indexes
                .definitions_by_query_name
                .get(name)
                .map_or(&[], Box::as_ref),
        )
    }

    /// Returns the fragment that registered one generic declaration.
    ///
    /// # Parameters
    ///
    /// - `id`: Process-local declaration identity.
    ///
    /// # Returns
    ///
    /// Returns the source fragment, or `None` when absent.
    #[must_use]
    #[inline]
    pub fn definition_source(&self, id: TypeDefinitionId) -> Option<&FragmentIdentity> {
        self.indexes.definition_fragments.get(&id)
    }

    /// Enumerates generic declarations together with their source fragments.
    ///
    /// # Returns
    ///
    /// Returns exact-size pairs of declaration and source identity.
    #[must_use]
    pub fn definitions_with_identity(
        &self,
    ) -> impl ExactSizeIterator<Item = (&'static TypeDefinitionDescriptor, &FragmentIdentity)> + '_ {
        self.definitions.iter().map(|definition| {
            let identity = self
                .indexes
                .definition_fragments
                .get(&definition.id())
                .expect("every frozen type definition has a source fragment");
            (*definition, identity)
        })
    }

    /// Enumerates registered roots together with their source fragments.
    ///
    /// # Returns
    ///
    /// Returns exact-size pairs of root descriptor and source identity.
    #[must_use]
    pub fn types_with_identity(
        &self,
    ) -> impl ExactSizeIterator<Item = (&'static TypeDescriptor, &FragmentIdentity)> + '_ {
        self.types.iter().map(|descriptor| {
            let identity = self
                .indexes
                .type_fragments
                .get(&descriptor.type_id())
                .expect("every frozen type has a source fragment");
            (*descriptor, identity)
        })
    }

    /// Returns the source fragment that registered one exact concrete type.
    ///
    /// # Parameters
    ///
    /// - `type_id`: Exact process-local type identity.
    ///
    /// # Returns
    ///
    /// Returns the source fragment, or `None` when absent.
    #[must_use]
    pub fn type_source(&self, type_id: TypeId) -> Option<&FragmentIdentity> {
        self.indexes.type_fragments.get(&type_id)
    }

    /// Returns the effective capabilities for one exact concrete descriptor.
    ///
    /// Registered targets borrow frozen facts without executing a provider.
    /// Unregistered monomorphs may initialize their intrinsic capability set.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Concrete reflected type whose effective capabilities are
    ///   requested.
    ///
    /// # Returns
    ///
    /// Returns the effective capability set for `descriptor`.
    ///
    /// # Errors
    ///
    /// Returns the complete conflict if intrinsic capability declarations use
    /// the same ID more than once. Failure never changes this snapshot.
    pub fn capabilities<'registry>(
        &'registry self,
        descriptor: &'registry TypeDescriptor,
    ) -> Result<&'registry TypeCapabilities, CapabilityConflict> {
        let type_id = descriptor.type_id();
        if let Some(capabilities) = self.indexes.capabilities_by_target.get(&type_id) {
            return Ok(capabilities);
        }
        if self.indexes.types_by_id.contains_key(&type_id) {
            return Ok(&self.empty_capabilities);
        }
        descriptor.declared_capabilities()
    }

    /// Returns capabilities only when `type_id` is a member of this snapshot.
    ///
    /// `None` means the type is not a snapshot member, even if capability
    /// facts were registered for it. `Some(empty)` means the type is a member
    /// without effective capability facts. This frozen lookup never executes
    /// an intrinsic capability provider.
    ///
    /// # Parameters
    ///
    /// - `type_id`: Exact process-local type identity to query.
    ///
    /// # Returns
    ///
    /// Returns the member's effective capability set, or `None` when the type
    /// is not a member of this snapshot.
    #[must_use]
    #[inline]
    pub fn member_capabilities(&self, type_id: TypeId) -> Option<&TypeCapabilities> {
        self.indexes.types_by_id.contains_key(&type_id).then(|| {
            self.indexes
                .capabilities_by_target
                .get(&type_id)
                .unwrap_or(&self.empty_capabilities)
        })
    }

    /// Retrieves an effective adapter matching the exact typed key.
    ///
    /// Returns `Ok(Some(adapter))` when found and `Ok(None)` when missing.
    /// Fact-only descriptors and adapter-contract mismatches return their
    /// corresponding errors. Use [`Self::capability_lookup`] to inspect all
    /// four lookup states directly. Intrinsic declaration conflicts return
    /// their conflict error.
    ///
    /// # Type Parameters
    ///
    /// - `A`: Expected adapter type associated with `key`.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Concrete reflected type to query.
    /// - `key`: Typed identity of the requested capability.
    ///
    /// # Errors
    ///
    /// Returns [`CapabilityAccessError::FactOnly`] when the descriptor has
    /// facts but no adapter for the requested capability, or
    /// [`CapabilityAccessError::AdapterTypeMismatch`] when the adapter type
    /// does not match the typed key. Returns an intrinsic conflict for an
    /// invalid unregistered descriptor.
    ///
    /// # Returns
    ///
    /// Returns the matching executable adapter, or `None` when the capability
    /// ID is missing.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use qubit_reflect::{ReflectRegistry, TypeDescriptor};
    /// use qubit_reflect::capability::clone_key;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let registry = ReflectRegistry::initialize()?;
    /// let adapter = registry.capability(TypeDescriptor::of::<String>(), clone_key())?;
    /// assert!(adapter.is_some());
    /// # Ok(())
    /// # }
    /// ```
    pub fn capability<'registry, A: 'static>(
        &'registry self,
        descriptor: &'registry TypeDescriptor,
        key: CapabilityKey<A>,
    ) -> Result<Option<&'registry A>, CapabilityAccessError> {
        self.capability_lookup(descriptor, key)
            .map_err(CapabilityAccessError::IntrinsicConflict)?
            .into_adapter()
    }

    /// Looks up one effective typed capability without collapsing diagnostic
    /// states into absence.
    ///
    /// # Type Parameters
    ///
    /// - `A`: Expected adapter type associated with `key`.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Concrete reflected type to query.
    /// - `key`: Typed identity of the requested capability.
    ///
    /// # Errors
    ///
    /// Returns an intrinsic conflict for an invalid unregistered descriptor.
    ///
    /// # Returns
    ///
    /// Returns the diagnostic lookup state for `key`.
    pub fn capability_lookup<'registry, A: 'static>(
        &'registry self,
        descriptor: &'registry TypeDescriptor,
        key: CapabilityKey<A>,
    ) -> Result<CapabilityLookup<'registry, A>, CapabilityConflict> {
        Ok(self.capabilities(descriptor)?.lookup(key))
    }

    /// Finds an effective concrete capability by textual ID.
    ///
    /// Returns `Ok(None)` for unmatched IDs, including invalid textual IDs,
    /// and `Err` when the intrinsic capability set cannot be formed.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Concrete reflected type to query.
    /// - `id`: Stable textual capability identifier.
    ///
    /// # Returns
    ///
    /// Returns the matching descriptor, or `None` when no descriptor has
    /// `id`.
    ///
    /// # Errors
    ///
    /// Returns the intrinsic capability conflict if the descriptor's provider
    /// cannot produce a valid capability set.
    pub fn capability_by_id<'registry>(
        &'registry self,
        descriptor: &'registry TypeDescriptor,
        id: &str,
    ) -> Result<Option<&'registry CapabilityDescriptor>, CapabilityConflict> {
        Ok(self.capabilities(descriptor)?.descriptor(id))
    }

    /// Returns the owned origin of an effective capability by textual ID.
    ///
    /// Unregistered descriptors may initialize their intrinsic capability
    /// provider while this method resolves the effective set.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Concrete reflected type to query.
    /// - `capability_id`: Stable textual capability identifier.
    ///
    /// # Returns
    ///
    /// Returns the capability origin, or `None` when no capability has
    /// `capability_id`.
    ///
    /// # Errors
    ///
    /// Returns the intrinsic capability conflict when the descriptor's
    /// provider cannot produce a valid capability set.
    pub fn capability_origin(
        &self,
        descriptor: &TypeDescriptor,
        capability_id: &str,
    ) -> Result<Option<CapabilityOrigin>, CapabilityConflict> {
        let capabilities = self.capabilities(descriptor)?;
        let Some(capability) = capabilities.descriptor(capability_id) else {
            return Ok(None);
        };
        let target = CapabilityTarget::Type(descriptor.type_id());
        let origin = self
            .indexes
            .capability_origins
            .get(&(target, *capability.id()))
            .cloned()
            .or_else(|| {
                (!self.indexes.types_by_id.contains_key(&descriptor.type_id())).then_some(CapabilityOrigin::Intrinsic {
                    type_id: descriptor.type_id(),
                })
            })
            .expect("every effective capability has a retained origin");
        Ok(Some(origin))
    }

    /// Returns the registration fragment that contributed a capability.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Concrete reflected type to query.
    /// - `capability_id`: Stable textual capability identifier.
    ///
    /// # Returns
    ///
    /// Returns the source fragment, or `None` when no registered capability
    /// matches.
    #[must_use]
    pub fn capability_source(&self, descriptor: &TypeDescriptor, capability_id: &str) -> Option<&FragmentIdentity> {
        let capabilities = self.indexes.capabilities_by_target.get(&descriptor.type_id())?;
        let capability = capabilities.descriptor(capability_id)?;
        self.indexes.capability_fragments.get(&(
            CapabilityTarget::Type(descriptor.type_id()),
            *capability.id(),
        ))
    }

    /// Returns the owned origin of a generic declaration capability by
    /// textual ID. Definition capabilities are always retained in the
    /// snapshot and therefore do not execute a provider here.
    ///
    /// # Parameters
    ///
    /// - `id`: Process-local generic declaration identity.
    /// - `capability_id`: Stable textual capability identifier.
    ///
    /// # Returns
    ///
    /// Returns the capability origin, or `None` when no matching capability
    /// exists.
    #[must_use]
    pub fn definition_capability_origin(&self, id: TypeDefinitionId, capability_id: &str) -> Option<CapabilityOrigin> {
        let capability = self.definition_capability_by_id(id, capability_id)?;
        let target = CapabilityTarget::TypeDefinition(id);
        Some(
            self.indexes
                .capability_origins
                .get(&(target, *capability.id()))
                .cloned()
                .expect("every effective definition capability has a retained origin"),
        )
    }

    /// Returns the registration fragment that contributed a generic capability.
    ///
    /// # Parameters
    ///
    /// - `id`: Process-local generic declaration identity.
    /// - `capability_id`: Stable textual capability identifier.
    ///
    /// # Returns
    ///
    /// Returns the source fragment, or `None` when no matching capability
    /// exists.
    #[must_use]
    pub fn definition_capability_source(&self, id: TypeDefinitionId, capability_id: &str) -> Option<&FragmentIdentity> {
        let capabilities = self.indexes.capabilities_by_definition.get(&id)?;
        let capability = capabilities.descriptor(capability_id)?;
        self.indexes.capability_fragments.get(&(
            CapabilityTarget::TypeDefinition(id),
            *capability.id(),
        ))
    }

    /// Returns the effective capabilities of one generic declaration.
    ///
    /// Returns `None` when the ID has neither definition membership nor
    /// capability facts. A capability-only target returns its effective facts
    /// even though [`Self::definition`] returns `None`; an empty capability
    /// set means the definition is a snapshot member with no effective facts.
    /// Use [`Self::definition`] when membership must be checked.
    ///
    /// # Parameters
    ///
    /// - `id`: Process-local generic declaration identity.
    #[must_use]
    pub fn definition_capabilities(&self, id: TypeDefinitionId) -> Option<&TypeCapabilities> {
        self.indexes.capabilities_by_definition.get(&id).or_else(|| {
            self.indexes
                .definitions_by_id
                .contains_key(&id)
                .then_some(&self.empty_capabilities)
        })
    }

    /// Returns capabilities only when `id` is a member of this snapshot.
    ///
    /// `None` means the definition is not a snapshot member, even if
    /// capability facts were registered for it. `Some(empty)` means the
    /// definition is a member without capability facts. Definition
    /// capabilities are frozen in the snapshot, so this lookup never executes
    /// a provider.
    ///
    /// # Parameters
    ///
    /// - `id`: Process-local generic definition identity to query.
    ///
    /// # Returns
    ///
    /// Returns the member's capability set, or `None` when the definition is
    /// not a member of this snapshot.
    #[must_use]
    #[inline]
    pub fn member_definition_capabilities(&self, id: TypeDefinitionId) -> Option<&TypeCapabilities> {
        self.indexes.definitions_by_id.contains_key(&id).then(|| {
            self.indexes
                .capabilities_by_definition
                .get(&id)
                .unwrap_or(&self.empty_capabilities)
        })
    }

    /// Retrieves one effective typed capability for a generic declaration.
    ///
    /// # Type Parameters
    ///
    /// - `A`: Expected adapter type associated with `key`.
    ///
    /// # Parameters
    ///
    /// - `id`: Process-local generic declaration identity.
    /// - `key`: Typed identity of the requested capability.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some(adapter))` when the capability facts contain an
    /// adapter for `key`. Returns `Ok(None)` when the ID has neither snapshot
    /// membership nor capability facts, or when its capability facts do not
    /// contain `key`'s ID. Membership is independent of capability lookup: a
    /// capability-only target can return an adapter while
    /// [`Self::definition`] returns `None`.
    ///
    /// ```
    /// use qubit_reflect::capability::{CapabilityDescriptor, CapabilityKey};
    /// use qubit_reflect::descriptor::{TypeDefinitionDescriptor, TypeDefinitionId};
    /// use qubit_reflect::expression::GenericDefinitionDescriptor;
    /// use qubit_reflect::identity::{CapabilityId, FragmentIdentity};
    /// use qubit_reflect::registry::RegistrySnapshotBuilder;
    ///
    /// struct Marker;
    /// let id = TypeDefinitionId::of::<Marker>();
    /// let generics = Box::leak(Box::new(GenericDefinitionDescriptor::new([], [])));
    /// let definition = Box::leak(Box::new(TypeDefinitionDescriptor::opaque(
    ///     id, "example::Record", "Record", generics,
    /// )));
    /// let key = CapabilityKey::<u32>::new(CapabilityId::new("example.record_adapter")?);
    /// let mut builder = RegistrySnapshotBuilder::new();
    /// builder.add_definition_capabilities(
    ///     definition,
    ///     vec![CapabilityDescriptor::with_adapter(key, 7)],
    ///     FragmentIdentity::new("example", "record", 1, 1, "capability", 1),
    /// );
    /// let registry = builder.build()?;
    ///
    /// assert!(registry.definition(id).is_none());
    /// assert_eq!(registry.definition_capability(id, key)?, Some(&7));
    /// assert!(registry.definition_capability_by_id(id, "example.record_adapter").is_some());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`CapabilityAccessError::FactOnly`] when the matching capability
    /// fact declares `key`'s adapter type but has no executable adapter.
    /// Returns [`CapabilityAccessError::AdapterTypeMismatch`] when the fact
    /// declares the same capability ID with a different adapter type.
    pub fn definition_capability<A: 'static>(
        &self,
        id: TypeDefinitionId,
        key: CapabilityKey<A>,
    ) -> Result<Option<&A>, CapabilityAccessError> {
        match self.definition_capabilities(id) {
            Some(capabilities) => capabilities.get(key),
            None => Ok(None),
        }
    }

    /// Finds one effective declaration capability by textual ID.
    ///
    /// # Parameters
    ///
    /// - `id`: Process-local generic declaration identity.
    /// - `capability_id`: Stable textual capability identifier.
    ///
    /// # Returns
    ///
    /// Returns the descriptor when the capability facts contain
    /// `capability_id`, including a fact with no executable adapter. Returns
    /// `None` when the ID has neither snapshot membership nor capability facts,
    /// or when the capability facts do not contain `capability_id`. The target
    /// need not be a snapshot member; use [`Self::definition`] to check
    /// membership separately.
    #[must_use]
    pub fn definition_capability_by_id(
        &self,
        id: TypeDefinitionId,
        capability_id: &str,
    ) -> Option<&CapabilityDescriptor> {
        self.definition_capabilities(id)?.descriptor(capability_id)
    }

    /// Enumerates registered concrete types that declare the capability ID.
    ///
    /// Fact-only capabilities and adapter-type mismatches are included so
    /// callers can inspect the complete lookup state. Capability-only targets
    /// are excluded because they are not members of this snapshot.
    ///
    /// # Type Parameters
    ///
    /// - `A`: Adapter contract required by `key`.
    ///
    /// # Parameters
    ///
    /// - `key`: Stable capability ID and expected adapter contract.
    ///
    /// # Returns
    ///
    /// Returns members in stable type-fragment order. Every returned lookup is
    /// `Found`, `FactOnly`, or `AdapterTypeMismatch`.
    pub fn type_capability_members<'registry, A: 'static>(
        &'registry self,
        key: CapabilityKey<A>,
    ) -> impl Iterator<Item = CapabilityMember<'registry, &'static TypeDescriptor, A>> + 'registry {
        self.types.iter().copied().filter_map(move |descriptor| {
            let type_id = descriptor.type_id();
            let target = CapabilityTarget::Type(type_id);
            let capabilities = self.indexes.capabilities_by_target.get(&type_id)?;
            let capability = capabilities.descriptor(key.id().as_str())?;
            let source = self
                .indexes
                .capability_fragments
                .get(&(target, *capability.id()))
                .expect("every effective type capability retains its source fragment");
            let origin = self
                .indexes
                .capability_origins
                .get(&(target, *capability.id()))
                .expect("every effective type capability retains its origin")
                .clone();
            Some(CapabilityMember::new(
                descriptor,
                capabilities.lookup(key),
                origin,
                source,
            ))
        })
    }

    /// Enumerates registered generic definitions that declare the capability
    /// ID.
    ///
    /// Fact-only capabilities and adapter-type mismatches are included so
    /// callers can inspect the complete lookup state.
    ///
    /// # Type Parameters
    ///
    /// - `A`: Adapter contract required by `key`.
    ///
    /// # Parameters
    ///
    /// - `key`: Stable capability ID and expected adapter contract.
    ///
    /// # Returns
    ///
    /// Returns members in stable definition-fragment order. Every returned
    /// lookup is `Found`, `FactOnly`, or `AdapterTypeMismatch`.
    pub fn definition_capability_members<'registry, A: 'static>(
        &'registry self,
        key: CapabilityKey<A>,
    ) -> impl Iterator<Item = CapabilityMember<'registry, &'static TypeDefinitionDescriptor, A>> + 'registry {
        self.definitions.iter().copied().filter_map(move |definition| {
            let definition_id = definition.id();
            let target = CapabilityTarget::TypeDefinition(definition_id);
            let capabilities = self.indexes.capabilities_by_definition.get(&definition_id)?;
            let capability = capabilities.descriptor(key.id().as_str())?;
            let source = self
                .indexes
                .capability_fragments
                .get(&(target, *capability.id()))
                .expect("every effective definition capability retains its source fragment");
            let origin = self
                .indexes
                .capability_origins
                .get(&(target, *capability.id()))
                .expect("every effective definition capability retains its origin")
                .clone();
            Some(CapabilityMember::new(
                definition,
                capabilities.lookup(key),
                origin,
                source,
            ))
        })
    }

    /// Returns registered capability targets absent from this snapshot's type
    /// membership, ordered by their source fragment identity.
    ///
    /// Matching uses the stable capability ID and includes adapter type
    /// mismatches. This query does not add targets to [`Self::types`].
    #[must_use]
    pub fn capability_only_type_targets(&self, capability_id: &str) -> Vec<(TypeId, &FragmentIdentity)> {
        let mut targets = self
            .indexes
            .capabilities_by_target
            .iter()
            .filter_map(|(type_id, capabilities)| {
                if self.indexes.types_by_id.contains_key(type_id) {
                    return None;
                }
                let capability = capabilities.descriptor(capability_id)?;
                let source = self
                    .indexes
                    .capability_fragments
                    .get(&(
                        CapabilityTarget::Type(*type_id),
                        *capability.id(),
                    ))
                    .expect("every effective capability has a retained source fragment");
                Some((*type_id, source))
            })
            .collect::<Vec<_>>();
        targets.sort_by(|left, right| left.1.cmp(right.1));
        targets
    }

    /// Returns generic definition targets with this capability that are not
    /// registered as reflected definitions. Capability identity alone is
    /// matched, so this includes fact-only capabilities and adapter types that
    /// do not match a caller's expected provider type.
    ///
    /// Results are ordered by the capability's source fragment. This audits
    /// the frozen registry facts only: it does not execute providers or add
    /// definitions to the registry.
    ///
    /// # Parameters
    ///
    /// - `capability_id`: Stable textual capability identifier.
    ///
    /// # Returns
    ///
    /// Returns each matching declaration identity with its source fragment.
    #[must_use]
    pub fn capability_only_definition_targets(
        &self,
        capability_id: &str,
    ) -> Vec<(TypeDefinitionId, &FragmentIdentity)> {
        let mut targets = self
            .indexes
            .capabilities_by_definition
            .iter()
            .filter_map(|(id, capabilities)| {
                if self.indexes.definitions_by_id.contains_key(id) {
                    return None;
                }
                let capability = capabilities.descriptor(capability_id)?;
                let source = self
                    .indexes
                    .capability_fragments
                    .get(&(
                        CapabilityTarget::TypeDefinition(*id),
                        *capability.id(),
                    ))
                    .expect("every effective definition capability has a retained source fragment");
                Some((*id, source))
            })
            .collect::<Vec<_>>();
        targets.sort_by(|left, right| left.1.cmp(right.1));
        targets
    }

    /// Returns every reflected implementation targeting `type_id`.
    ///
    /// The slice is empty when no linked implementation fragment targets the
    /// exact root. Its order is the registry's stable fragment order, so a
    /// caller can pass it directly to [`ImplDescriptor::lookup_method`].
    ///
    /// # Parameters
    ///
    /// - `type_id`: Exact process-local identity of the target type.
    ///
    /// # Returns
    ///
    /// Returns matching implementations in stable source-fragment order.
    #[must_use]
    #[inline]
    pub fn implementations(&self, type_id: TypeId) -> &[&'static ImplDescriptor] {
        self.indexes.impls_by_target.get(&type_id).map_or(&[], Box::as_ref)
    }

    /// Enumerates statically registered generic, blanket, and constrained impl
    /// declarations in stable source-fragment order.
    ///
    /// Generic and blanket definitions appear here even when they have no
    /// explicitly registered concrete specialization.
    #[must_use]
    #[inline]
    pub fn impl_definitions(&self) -> &[&'static ImplDefinitionDescriptor] {
        &self.impl_definitions
    }

    /// Finds impl declarations whose symbolic target exactly equals `target`.
    ///
    /// Diagnostic-only text does not participate because
    /// [`TypeExpression`] equality is structural.
    ///
    /// # Parameters
    ///
    /// - `target`: Structural symbolic type expression to match.
    ///
    /// # Returns
    ///
    /// Returns matching declarations in stable source-fragment order.
    #[must_use]
    #[inline]
    pub fn find_impl_definitions_by_target(&self, target: &TypeExpression) -> ImplDefinitionCandidates {
        ImplDefinitionCandidates::new(
            self.impl_definitions
                .iter()
                .copied()
                .filter(|definition| definition.target_type() == target)
                .collect(),
        )
    }

    /// Returns the target's frozen deterministic effective method view.
    ///
    /// Repeated calls borrow the same registry-owned view without allocation.
    /// An unregistered target borrows one stable empty view.
    ///
    /// # Parameters
    ///
    /// - `type_id`: Exact process-local identity of the target type.
    ///
    /// # Returns
    ///
    /// Returns the frozen effective view, or the stable empty view when absent.
    #[must_use]
    #[inline]
    pub fn effective_view(&self, type_id: TypeId) -> &EffectiveTypeView {
        self.indexes
            .effective_views_by_target
            .get(&type_id)
            .unwrap_or(&self.empty_effective_view)
    }

    /// Returns the trait linked to an impl definition in this snapshot.
    ///
    /// `None` means the definition is absent or describes an inherent impl.
    /// Links belong to the snapshot and never mutate shared declaration facts.
    ///
    /// # Parameters
    ///
    /// - `definition`: Impl declaration whose resolved trait link is requested.
    ///
    /// # Returns
    ///
    /// Returns the linked trait declaration, or `None` when no trait is linked.
    #[must_use]
    #[inline]
    pub fn impl_definition_trait(
        &self,
        definition: &ImplDefinitionDescriptor,
    ) -> Option<&'static TraitDefinitionDescriptor> {
        self.indexes
            .impl_definition_traits
            .get(definition.fragment_identity())
            .copied()
    }

    /// Finds a reflected or external trait declaration by its process-local
    /// identity.
    ///
    /// `None` means no linked registration fragment declared the requested
    /// trait.
    ///
    /// # Parameters
    ///
    /// - `trait_id`: Process-local identity of the trait declaration.
    ///
    /// # Returns
    ///
    /// Returns the registered trait declaration, or `None` when absent.
    #[must_use]
    #[inline]
    pub fn trait_definition(&self, trait_id: &TraitId) -> Option<&'static TraitDefinitionDescriptor> {
        self.indexes.traits_by_id.get(trait_id).copied()
    }

    /// Finds the sole reflected trait declaration with a diagnostic Rust path.
    ///
    /// `None` means no linked reflected trait declaration has the exact path,
    /// or the path is ambiguous across linked fragments.
    ///
    /// # Parameters
    ///
    /// - `rust_path`: Exact diagnostic Rust path to find.
    ///
    /// # Returns
    ///
    /// Returns the sole matching reflected declaration, or `None` when absent
    /// or ambiguous.
    #[must_use]
    #[inline]
    pub fn trait_definition_by_path(&self, rust_path: &str) -> Option<&'static TraitDefinitionDescriptor> {
        self.find_trait_definitions_by_path(rust_path).only()
    }

    /// Finds every trait declaration with a diagnostic Rust path in stable
    /// order.
    ///
    /// # Parameters
    ///
    /// - `rust_path`: Exact diagnostic Rust path to find.
    ///
    /// # Returns
    ///
    /// Returns all matching declarations in stable order.
    pub fn find_trait_definitions_by_path(&self, rust_path: &str) -> TraitCandidates<'_> {
        TraitCandidates::new(self.indexes.traits_by_rust_path.get(rust_path).map_or(&[], Box::as_ref))
    }
}
