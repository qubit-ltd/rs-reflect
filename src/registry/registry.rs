// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Frozen registry snapshot and public deterministic lookup views.

use std::any::TypeId;
use std::sync::OnceLock;

use crate::capability::CapabilityDescriptor;
use crate::capability::CapabilityKey;
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
use crate::registry::indexes::RegistryIndexes;
use crate::registry::registry_builder::build_inventory_registry;
use crate::registry::registry_builder::initialize_cached;

/// A borrowed, deterministic set of type-descriptor name matches.
#[derive(Clone, Copy, Debug)]
pub struct TypeCandidates<'registry> {
    descriptors: &'registry [&'static TypeDescriptor],
}

/// A borrowed, deterministic set of generic declaration matches.
#[derive(Clone, Copy, Debug)]
pub struct TypeDefinitionCandidates<'registry> {
    descriptors: &'registry [&'static TypeDefinitionDescriptor],
}

impl<'registry> TypeDefinitionCandidates<'registry> {
    /// Creates a candidate view over one registry-owned slice.
    pub(crate) const fn new(descriptors: &'registry [&'static TypeDefinitionDescriptor]) -> Self {
        Self { descriptors }
    }

    /// Returns candidates in stable fragment order.
    #[must_use]
    pub fn iter(self) -> impl ExactSizeIterator<Item = &'static TypeDefinitionDescriptor> + 'registry {
        self.descriptors.iter().copied()
    }

    /// Returns the number of matching declarations.
    #[must_use]
    pub const fn len(self) -> usize {
        self.descriptors.len()
    }

    /// Returns whether no declaration matched.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.descriptors.is_empty()
    }

    /// Returns the sole matching declaration, or `None` when absent or
    /// ambiguous.
    #[must_use]
    pub fn only(self) -> Option<&'static TypeDefinitionDescriptor> {
        (self.descriptors.len() == 1).then(|| self.descriptors[0])
    }
}

impl<'registry> IntoIterator for TypeDefinitionCandidates<'registry> {
    type Item = &'static TypeDefinitionDescriptor;
    type IntoIter = std::iter::Copied<std::slice::Iter<'registry, &'static TypeDefinitionDescriptor>>;

    /// Iterates over declarations in stable fragment order.
    fn into_iter(self) -> Self::IntoIter {
        self.descriptors.iter().copied()
    }
}

impl<'registry> TypeCandidates<'registry> {
    /// Creates a borrowed candidate view over a registry-owned slice.
    pub(crate) const fn new(descriptors: &'registry [&'static TypeDescriptor]) -> Self {
        Self { descriptors }
    }

    /// Returns candidates in stable fragment order.
    #[must_use]
    #[inline(always)]
    pub fn iter(self) -> impl ExactSizeIterator<Item = &'static TypeDescriptor> + 'registry {
        self.descriptors.iter().copied()
    }

    /// Returns the number of matching descriptors.
    #[must_use]
    #[inline(always)]
    pub const fn len(self) -> usize {
        self.descriptors.len()
    }

    /// Returns whether no descriptor matched the requested name.
    #[must_use]
    #[inline(always)]
    pub const fn is_empty(self) -> bool {
        self.descriptors.is_empty()
    }
}

impl<'registry> IntoIterator for TypeCandidates<'registry> {
    type Item = &'static TypeDescriptor;
    type IntoIter = std::iter::Copied<std::slice::Iter<'registry, &'static TypeDescriptor>>;

    /// Iterates over candidates in stable fragment order.
    fn into_iter(self) -> Self::IntoIter {
        self.descriptors.iter().copied()
    }
}

/// An ordered borrowed view over trait declarations sharing one diagnostic
/// path.
#[derive(Clone, Copy, Debug)]
pub struct TraitCandidates<'registry> {
    descriptors: &'registry [&'static TraitDefinitionDescriptor],
}

impl<'registry> TraitCandidates<'registry> {
    /// Creates a borrowed candidate view over a registry-owned slice.
    pub(crate) const fn new(descriptors: &'registry [&'static TraitDefinitionDescriptor]) -> Self {
        Self { descriptors }
    }

    /// Returns candidates in stable fragment order.
    #[must_use]
    #[inline(always)]
    pub fn iter(self) -> impl ExactSizeIterator<Item = &'static TraitDefinitionDescriptor> + 'registry {
        self.descriptors.iter().copied()
    }

    /// Returns the number of matching declarations.
    #[must_use]
    #[inline(always)]
    pub const fn len(self) -> usize {
        self.descriptors.len()
    }

    /// Returns whether no declarations match.
    #[must_use]
    #[inline(always)]
    pub const fn is_empty(self) -> bool {
        self.descriptors.is_empty()
    }

    /// Returns the sole matching declaration, rejecting ambiguous path lookups.
    #[must_use]
    #[inline(always)]
    pub fn only(self) -> Option<&'static TraitDefinitionDescriptor> {
        (self.descriptors.len() == 1).then(|| self.descriptors[0])
    }
}

impl<'registry> IntoIterator for TraitCandidates<'registry> {
    type Item = &'static TraitDefinitionDescriptor;
    type IntoIter = std::iter::Copied<std::slice::Iter<'registry, &'static TraitDefinitionDescriptor>>;

    /// Iterates over candidates in stable fragment order.
    fn into_iter(self) -> Self::IntoIter {
        self.descriptors.iter().copied()
    }
}

/// A deterministic borrowed set of impl definitions matching one symbolic
/// target expression.
#[derive(Clone, Debug)]
pub struct ImplDefinitionCandidates {
    descriptors: Box<[&'static ImplDefinitionDescriptor]>,
}

impl ImplDefinitionCandidates {
    /// Iterates over matching definitions in stable fragment order.
    #[must_use]
    #[inline(always)]
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &'static ImplDefinitionDescriptor> + '_ {
        self.descriptors.iter().copied()
    }

    /// Returns the number of matching definitions.
    #[must_use]
    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.descriptors.len()
    }

    /// Returns whether no impl definition has the requested target.
    #[must_use]
    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.descriptors.is_empty()
    }
}

impl IntoIterator for ImplDefinitionCandidates {
    type Item = &'static ImplDefinitionDescriptor;
    type IntoIter = std::vec::IntoIter<&'static ImplDefinitionDescriptor>;

    /// Iterates over matching definitions in stable fragment order.
    fn into_iter(self) -> Self::IntoIter {
        self.descriptors.into_vec().into_iter()
    }
}

/// The immutable process-wide snapshot of linked reflection fragments.
///
/// # Examples
///
/// ```
/// use std::any::TypeId;
/// use qubit_reflect::registry::ReflectRegistry;
///
/// let registry = ReflectRegistry::initialize()?;
/// let _registered_u8 = registry.get(TypeId::of::<u8>());
/// # Ok::<(), qubit_reflect::error::RegistryError>(())
/// ```
#[derive(Debug)]
pub struct ReflectRegistry {
    pub(super) types: Box<[&'static TypeDescriptor]>,
    pub(super) definitions: Box<[&'static TypeDefinitionDescriptor]>,
    pub(super) impl_definitions: Box<[&'static ImplDefinitionDescriptor]>,
    pub(super) indexes: RegistryIndexes,
    pub(super) empty_effective_view: EffectiveTypeView,
    pub(super) empty_capabilities: TypeCapabilities,
}

impl ReflectRegistry {
    /// Initializes and returns the process-wide immutable registry snapshot.
    ///
    /// All linked fragments are sorted, materialized, and validated before a
    /// snapshot is published. Both success and [`RegistryError`] are cached;
    /// concurrent callers therefore observe the same immutable result.
    pub fn initialize() -> Result<&'static Self, RegistryError> {
        static REGISTRY: OnceLock<Result<ReflectRegistry, RegistryError>> = OnceLock::new();
        initialize_cached(&REGISTRY, build_inventory_registry)
    }

    /// Looks up one exact concrete registered type.
    ///
    /// `None` means no linked static fragment registered `type_id`.
    #[must_use]
    pub fn get(&self, type_id: TypeId) -> Option<&'static TypeDescriptor> {
        self.indexes.types_by_id.get(&type_id).copied()
    }

    /// Finds every descriptor with the diagnostic Rust type name `name`.
    ///
    /// The returned view is empty when no descriptor matches and preserves the
    /// registry's stable fragment order when the name is ambiguous.
    pub fn find_by_type_name(&self, name: &str) -> TypeCandidates<'_> {
        TypeCandidates::new(self.indexes.types_by_type_name.get(name).map_or(&[], Box::as_ref))
    }

    /// Finds every descriptor with the reflection query name `name`.
    ///
    /// The returned view is empty when no descriptor matches and preserves the
    /// registry's stable fragment order when the name is ambiguous.
    pub fn find_by_query_name(&self, name: &str) -> TypeCandidates<'_> {
        TypeCandidates::new(self.indexes.types_by_query_name.get(name).map_or(&[], Box::as_ref))
    }

    /// Enumerates all statically registered roots in stable fragment order.
    #[must_use]
    #[inline(always)]
    pub fn types(&self) -> &[&'static TypeDescriptor] {
        &self.types
    }

    /// Enumerates all registered generic type declarations in fragment order.
    #[must_use]
    pub fn definitions(&self) -> &[&'static TypeDefinitionDescriptor] {
        &self.definitions
    }

    /// Looks up one generic declaration by its process-local identity.
    #[must_use]
    pub fn definition(&self, id: TypeDefinitionId) -> Option<&'static TypeDefinitionDescriptor> {
        self.indexes.definitions_by_id.get(&id).copied()
    }

    /// Finds generic declarations with the exact Rust source path.
    pub fn find_definitions_by_rust_path(&self, path: &str) -> TypeDefinitionCandidates<'_> {
        TypeDefinitionCandidates::new(self.indexes.definitions_by_rust_path.get(path).map_or(&[], Box::as_ref))
    }

    /// Finds generic declarations with the exact reflection query name.
    pub fn find_definitions_by_query_name(&self, name: &str) -> TypeDefinitionCandidates<'_> {
        TypeDefinitionCandidates::new(
            self.indexes
                .definitions_by_query_name
                .get(name)
                .map_or(&[], Box::as_ref),
        )
    }

    /// Returns the fragment that registered one generic declaration.
    #[must_use]
    pub fn definition_source(&self, id: TypeDefinitionId) -> Option<&FragmentIdentity> {
        self.indexes.definition_fragments.get(&id)
    }

    /// Enumerates generic declarations together with their source fragments.
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
    #[must_use]
    pub fn type_source(&self, type_id: TypeId) -> Option<&FragmentIdentity> {
        self.indexes.type_fragments.get(&type_id)
    }

    /// Returns the effective capabilities for one exact concrete descriptor.
    ///
    /// Registered targets borrow the frozen merged set. An unregistered
    /// generic monomorph borrows its generated intrinsic set.
    #[must_use]
    pub fn capabilities<'registry>(
        &'registry self,
        descriptor: &'registry TypeDescriptor,
    ) -> &'registry TypeCapabilities {
        self.indexes
            .capabilities_by_target
            .get(&descriptor.type_id())
            .unwrap_or_else(|| descriptor.declared_capabilities())
    }

    /// Retrieves one effective typed capability for a concrete descriptor.
    #[must_use]
    pub fn capability<'registry, A: 'static>(
        &'registry self,
        descriptor: &'registry TypeDescriptor,
        key: CapabilityKey<A>,
    ) -> Option<&'registry A> {
        self.indexes
            .capabilities_by_target
            .get(&descriptor.type_id())
            .unwrap_or_else(|| descriptor.declared_capabilities())
            .get(key)
    }

    /// Finds one effective concrete capability by textual ID.
    #[must_use]
    pub fn capability_by_id<'registry>(
        &'registry self,
        descriptor: &'registry TypeDescriptor,
        id: &str,
    ) -> Option<&'registry CapabilityDescriptor> {
        self.capabilities(descriptor).descriptor(id)
    }

    /// Returns the effective capabilities of one generic declaration.
    #[must_use]
    pub fn definition_capabilities(&self, id: TypeDefinitionId) -> &TypeCapabilities {
        self.indexes
            .capabilities_by_definition
            .get(&id)
            .unwrap_or(&self.empty_capabilities)
    }

    /// Retrieves one effective typed capability for a generic declaration.
    #[must_use]
    pub fn definition_capability<A: 'static>(&self, id: TypeDefinitionId, key: CapabilityKey<A>) -> Option<&A> {
        self.definition_capabilities(id).get(key)
    }

    /// Finds one effective declaration capability by textual ID.
    #[must_use]
    pub fn definition_capability_by_id(
        &self,
        id: TypeDefinitionId,
        capability_id: &str,
    ) -> Option<&CapabilityDescriptor> {
        self.definition_capabilities(id).descriptor(capability_id)
    }

    /// Enumerates registered roots carrying the exact typed capability key.
    pub fn types_with_capability<A: 'static>(
        &self,
        key: CapabilityKey<A>,
    ) -> impl Iterator<Item = &'static TypeDescriptor> + '_ {
        self.types
            .iter()
            .copied()
            .filter(move |descriptor| self.capabilities(descriptor).contains(key))
    }

    /// Enumerates generic declarations carrying the exact typed capability.
    pub fn definitions_with_capability<A: 'static>(
        &self,
        key: CapabilityKey<A>,
    ) -> impl Iterator<Item = &'static TypeDefinitionDescriptor> + '_ {
        self.definitions
            .iter()
            .copied()
            .filter(move |definition| self.definition_capabilities(definition.id()).contains(key))
    }

    /// Returns every reflected implementation targeting `type_id`.
    ///
    /// The slice is empty when no linked implementation fragment targets the
    /// exact root. Its order is the registry's stable fragment order, so a
    /// caller can pass it directly to [`ImplDescriptor::lookup_method`].
    pub fn implementations(&self, type_id: TypeId) -> &[&'static ImplDescriptor] {
        self.indexes.impls_by_target.get(&type_id).map_or(&[], Box::as_ref)
    }

    /// Enumerates statically registered generic, blanket, and constrained impl
    /// declarations in stable source-fragment order.
    ///
    /// Generic and blanket definitions appear here even when they have no
    /// explicitly registered concrete specialization.
    #[must_use]
    #[inline(always)]
    pub fn impl_definitions(&self) -> &[&'static ImplDefinitionDescriptor] {
        &self.impl_definitions
    }

    /// Finds impl declarations whose symbolic target exactly equals `target`.
    ///
    /// Diagnostic-only text does not participate because
    /// [`TypeExpression`] equality is structural.
    #[must_use]
    pub fn find_impl_definitions_by_target(&self, target: &TypeExpression) -> ImplDefinitionCandidates {
        ImplDefinitionCandidates {
            descriptors: self
                .impl_definitions
                .iter()
                .copied()
                .filter(|definition| definition.target_type() == target)
                .collect(),
        }
    }

    /// Returns the target's frozen deterministic effective method view.
    ///
    /// Repeated calls borrow the same registry-owned view without allocation.
    /// An unregistered target borrows one stable empty view.
    #[must_use]
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
    #[must_use]
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
    #[must_use]
    pub fn trait_definition(&self, trait_id: &TraitId) -> Option<&'static TraitDefinitionDescriptor> {
        self.indexes.traits_by_id.get(trait_id).copied()
    }

    /// Finds the sole reflected trait declaration with a diagnostic Rust path.
    ///
    /// `None` means no linked reflected trait declaration has the exact path,
    /// or the path is ambiguous across linked fragments.
    #[must_use]
    pub fn trait_definition_by_path(&self, rust_path: &str) -> Option<&'static TraitDefinitionDescriptor> {
        self.find_trait_definitions_by_path(rust_path).only()
    }

    /// Finds every trait declaration with a diagnostic Rust path in stable
    /// order.
    pub fn find_trait_definitions_by_path(&self, rust_path: &str) -> TraitCandidates<'_> {
        TraitCandidates::new(self.indexes.traits_by_rust_path.get(rust_path).map_or(&[], Box::as_ref))
    }
}
