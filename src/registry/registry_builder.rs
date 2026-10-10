// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Transactional validation and construction of frozen registry snapshots.

use std::any::TypeId;
use std::collections::HashMap;
use std::sync::OnceLock;

use crate::capability::CapabilityConflict;
use crate::capability::CapabilityDescriptor;
use crate::capability::CapabilityOrigin;
use crate::capability::TypeCapabilities;
use crate::descriptor::AppliedTraitId;
use crate::descriptor::ImplDefinitionDescriptor;
use crate::descriptor::ImplDescriptor;
use crate::descriptor::ImplKind;
use crate::descriptor::TraitDefinitionDescriptor;
use crate::descriptor::TraitId;
use crate::descriptor::TypeDefinitionDescriptor;
use crate::descriptor::TypeDefinitionId;
use crate::descriptor::TypeDescriptor;
use crate::error::RegistryError;
use crate::identity::CapabilityId;
use crate::identity::ExternalTraitId;
use crate::identity::FragmentIdentity;
use crate::registry::EffectiveTypeView;
use crate::registry::fragment::CapabilityRegistration;
use crate::registry::fragment::CapabilityTarget;
use crate::registry::fragment::FragmentPayload;
use crate::registry::fragment::RegistrationFragment;
use crate::registry::indexes::RegistryIndexes;
use crate::registry::internal::BuiltFragment;
use crate::registry::internal::MaterializedFragment;
use crate::registry::internal::PendingFragment;
use crate::registry::reflect_registry::ReflectRegistry;

/// Accumulates validated payloads without exposing partial registry state.
#[derive(Default)]
struct RegistryBuilder {
    /// Unique concrete type descriptors in deterministic input order.
    types: Vec<&'static TypeDescriptor>,
    /// Concrete type identity to descriptor and first contributing fragment.
    types_by_id: HashMap<TypeId, (&'static TypeDescriptor, FragmentIdentity)>,
    /// Generic type declarations in deterministic input order.
    definitions: Vec<&'static TypeDefinitionDescriptor>,
    /// Generic declaration identity to descriptor and first fragment.
    definitions_by_id: HashMap<TypeDefinitionId, (&'static TypeDefinitionDescriptor, FragmentIdentity)>,
    /// Trait declarations keyed by their stable identity.
    traits_by_id: HashMap<TraitId, &'static TraitDefinitionDescriptor>,
    /// First fragment that contributed each trait declaration.
    trait_fragments: HashMap<TraitId, FragmentIdentity>,
    /// External trait declarations keyed by external identity.
    external_traits: HashMap<ExternalTraitId, (&'static TraitDefinitionDescriptor, FragmentIdentity)>,
    /// Unique concrete implementations keyed by target and applied trait.
    trait_impls: HashMap<(TypeId, AppliedTraitId), FragmentIdentity>,
    /// Resolved trait declaration for each trait implementation fragment.
    impl_definition_traits: HashMap<FragmentIdentity, &'static TraitDefinitionDescriptor>,
    /// Source-level impl declarations in deterministic input order.
    impl_definitions: Vec<&'static ImplDefinitionDescriptor>,
    /// Concrete impl descriptors grouped by target type identity.
    impls_by_target: HashMap<TypeId, Vec<&'static ImplDescriptor>>,
    /// Effective capability facts and their first contributing fragment.
    capabilities: HashMap<(CapabilityTarget, CapabilityId), (CapabilityDescriptor, FragmentIdentity)>,
    /// Origin classification for every effective capability.
    capability_origins: HashMap<(CapabilityTarget, CapabilityId), CapabilityOrigin>,
    /// Candidate intrinsic capabilities awaiting conflict validation.
    intrinsic_candidates: HashMap<TypeId, (&'static TypeDescriptor, FragmentIdentity)>,
}

impl RegistryBuilder {
    /// Validates one materialized fragment and records its payload privately.
    ///
    /// # Parameters
    ///
    /// - `built`: Materialized fragment with validated static identity.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after recording the payload.
    ///
    /// # Errors
    ///
    /// Returns a conflict when another fragment claims incompatible facts.
    fn push(&mut self, built: BuiltFragment) -> Result<(), RegistryError> {
        match built.payload {
            FragmentPayload::Type(descriptor) => self.push_type(descriptor, &built.identity)?,
            FragmentPayload::TypeDefinition(descriptor) => self.push_definition(descriptor, &built.identity)?,
            FragmentPayload::Trait(descriptor) => self.push_trait(descriptor, &built.identity)?,
            FragmentPayload::ImplDefinition(descriptor) => {
                if descriptor.fragment_identity() != &built.identity {
                    return Err(RegistryError::identity_conflict(
                        descriptor.fragment_identity().clone(),
                        built.identity.clone(),
                    ));
                }
                self.impl_definitions.push(descriptor);
            }
            FragmentPayload::Impl(descriptor) => self.push_impl(descriptor, &built.identity)?,
            FragmentPayload::Capability(registration) => {
                self.push_capability_registration(&registration, &built.identity)?;
            }
        }
        Ok(())
    }

    /// Adds one unique source-level generic type declaration.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Generic declaration to register.
    /// - `identity`: Source fragment contributing the declaration.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after insertion.
    ///
    /// # Errors
    ///
    /// Returns an identity conflict when the declaration ID is already present.
    fn push_definition(
        &mut self,
        descriptor: &'static TypeDefinitionDescriptor,
        identity: &FragmentIdentity,
    ) -> Result<(), RegistryError> {
        if let Some((_, first)) = self.definitions_by_id.get(&descriptor.id()) {
            return Err(RegistryError::identity_conflict(first.clone(), identity.clone()));
        }
        self.definitions.push(descriptor);
        self.definitions_by_id
            .insert(descriptor.id(), (descriptor, identity.clone()));
        Ok(())
    }

    /// Adds one unique concrete root descriptor.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Concrete root to register.
    /// - `identity`: Source fragment contributing the root.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after insertion and intrinsic-provider queuing.
    ///
    /// # Errors
    ///
    /// Returns an identity conflict when the root type is already present.
    fn push_type(
        &mut self,
        descriptor: &'static TypeDescriptor,
        identity: &FragmentIdentity,
    ) -> Result<(), RegistryError> {
        if let Some((_, first)) = self.types_by_id.get(&descriptor.type_id()) {
            return Err(RegistryError::identity_conflict(first.clone(), identity.clone()));
        }
        self.types.push(descriptor);
        self.types_by_id
            .insert(descriptor.type_id(), (descriptor, identity.clone()));
        self.queue_intrinsic_capabilities(descriptor, identity);
        Ok(())
    }

    /// Adds one capability registration and the target's intrinsic facts.
    ///
    /// # Parameters
    ///
    /// - `registration`: Capability payload and its concrete or generic target.
    /// - `identity`: Source fragment contributing the registration.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after recording all capability facts.
    ///
    /// # Errors
    ///
    /// Returns a capability conflict for a duplicate target and ID.
    fn push_capability_registration(
        &mut self,
        registration: &CapabilityRegistration,
        identity: &FragmentIdentity,
    ) -> Result<(), RegistryError> {
        if let Some(descriptor) = registration.type_descriptor() {
            self.queue_intrinsic_capabilities(descriptor, identity);
        }
        for descriptor in registration.descriptors() {
            self.push_capability(registration.target(), descriptor.clone(), identity)?;
        }
        Ok(())
    }

    /// Defers one descriptor's intrinsic facts until concrete membership is
    /// known.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Root whose intrinsic capability provider will run.
    /// - `identity`: Fragment that first caused the root to be inspected.
    fn queue_intrinsic_capabilities(&mut self, descriptor: &'static TypeDescriptor, identity: &FragmentIdentity) {
        self.intrinsic_candidates
            .entry(descriptor.type_id())
            .or_insert((descriptor, identity.clone()));
    }

    /// Resolves intrinsic facts after the snapshot's concrete members are
    /// known.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after every queued provider has been resolved.
    ///
    /// # Errors
    ///
    /// Returns the first intrinsic capability conflict.
    fn resolve_intrinsic_capabilities(&mut self) -> Result<(), RegistryError> {
        let mut candidates = self.intrinsic_candidates.drain().collect::<Vec<_>>();
        candidates.sort_by(|(_, (_, left_source)), (_, (_, right_source))| left_source.cmp(right_source));
        for (type_id, (descriptor, first_trigger)) in candidates {
            let source = self
                .types_by_id
                .get(&type_id)
                .map_or(first_trigger, |(_, type_source)| type_source.clone());
            let capabilities = descriptor.declared_capabilities().map_err(|error| {
                RegistryError::intrinsic_capability_conflict_with_target(
                    source.clone(),
                    CapabilityTarget::Type(type_id),
                    error,
                )
            })?;
            self.push_intrinsic_descriptors(type_id, &source, capabilities.descriptors())?;
        }
        Ok(())
    }

    /// Adds one validated intrinsic capability with its selected source.
    ///
    /// # Parameters
    ///
    /// - `type_id`: Concrete target receiving the capabilities.
    /// - `source`: Fragment selected as the source of intrinsic facts.
    /// - `capabilities`: Validated intrinsic descriptors.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after recording all descriptors.
    ///
    /// # Errors
    ///
    /// Returns a conflict when a capability ID is already registered.
    fn push_intrinsic_descriptors(
        &mut self,
        type_id: TypeId,
        source: &FragmentIdentity,
        capabilities: &[CapabilityDescriptor],
    ) -> Result<(), RegistryError> {
        for capability in capabilities {
            self.push_capability_with_origin(
                CapabilityTarget::Type(type_id),
                capability.clone(),
                source,
                CapabilityOrigin::Intrinsic { type_id },
            )?;
        }
        Ok(())
    }

    /// Adds one capability while retaining the source fragment that claimed it.
    ///
    /// # Parameters
    ///
    /// - `target`: Concrete or generic capability target.
    /// - `descriptor`: Capability fact to record.
    /// - `identity`: Fragment that contributed the fact.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after recording the capability.
    ///
    /// # Errors
    ///
    /// Returns a conflict when the same target and ID already exist.
    fn push_capability(
        &mut self,
        target: CapabilityTarget,
        descriptor: CapabilityDescriptor,
        identity: &FragmentIdentity,
    ) -> Result<(), RegistryError> {
        self.push_capability_with_origin(
            target,
            descriptor,
            identity,
            CapabilityOrigin::Registered {
                source: identity.clone(),
            },
        )
    }

    /// Adds one capability while retaining its source and semantic origin.
    ///
    /// # Parameters
    ///
    /// - `target`: Concrete or generic capability target.
    /// - `descriptor`: Capability fact to record.
    /// - `identity`: Fragment that contributed the fact.
    /// - `origin`: Intrinsic or registered origin retained for lookup.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after recording the capability.
    ///
    /// # Errors
    ///
    /// Returns a conflict when the same target and ID already exist.
    fn push_capability_with_origin(
        &mut self,
        target: CapabilityTarget,
        descriptor: CapabilityDescriptor,
        identity: &FragmentIdentity,
        origin: CapabilityOrigin,
    ) -> Result<(), RegistryError> {
        let key = (target, *descriptor.id());
        if let Some((first_descriptor, first_identity)) = self.capabilities.get(&key) {
            let conflict = CapabilityConflict::from_same_id(first_descriptor, &descriptor);
            return Err(RegistryError::capability_conflict_with_details(
                first_identity.clone(),
                identity.clone(),
                target,
                conflict,
            ));
        }
        self.capabilities.insert(key, (descriptor, identity.clone()));
        self.capability_origins.insert(key, origin);
        Ok(())
    }

    /// Adds one trait descriptor and audits reflected and external identities.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Trait declaration to register.
    /// - `identity`: Source fragment contributing the declaration.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after registering the declaration.
    ///
    /// # Errors
    ///
    /// Returns an identity conflict for a duplicate reflected ID or
    /// incompatible external declaration.
    fn push_trait(
        &mut self,
        descriptor: &'static TraitDefinitionDescriptor,
        identity: &FragmentIdentity,
    ) -> Result<(), RegistryError> {
        let definition_id = descriptor.trait_id().clone();
        if let TraitId::External(external_id) = &definition_id {
            if let Some((first_descriptor, first_identity)) = self.external_traits.get(external_id) {
                if !descriptor.is_compatible_with(first_descriptor) {
                    return Err(RegistryError::external_trait_id_conflict(
                        first_identity.clone(),
                        identity.clone(),
                    ));
                }
            } else {
                self.external_traits
                    .insert(external_id.clone(), (descriptor, identity.clone()));
            }
        } else if let Some(first) = self.trait_fragments.get(&definition_id) {
            return Err(RegistryError::identity_conflict(first.clone(), identity.clone()));
        }
        self.trait_fragments
            .entry(definition_id.clone())
            .or_insert_with(|| identity.clone());
        self.traits_by_id.entry(definition_id).or_insert(descriptor);
        Ok(())
    }

    /// Adds one impl after validating its outer, definition, and target
    /// identities.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Concrete impl application to register.
    /// - `identity`: Source fragment contributing the application.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after recording the implementation.
    ///
    /// # Errors
    ///
    /// Returns an identity conflict for a repeated concrete trait impl.
    fn push_impl(
        &mut self,
        descriptor: &'static ImplDescriptor,
        identity: &FragmentIdentity,
    ) -> Result<(), RegistryError> {
        let definition_identity = descriptor.definition().fragment_identity();
        if definition_identity != identity && descriptor.arguments().is_empty() {
            return Err(RegistryError::identity_conflict(
                definition_identity.clone(),
                identity.clone(),
            ));
        }

        let target_type_id = descriptor.target_type().type_id();
        if let Some(implemented_trait) = descriptor.implemented_trait() {
            let key = (target_type_id, implemented_trait.trait_id().clone());
            if let Some(first) = self.trait_impls.get(&key) {
                return Err(RegistryError::identity_conflict(first.clone(), identity.clone()));
            }
            self.trait_impls.insert(key, identity.clone());
        }
        self.impls_by_target.entry(target_type_id).or_default().push(descriptor);
        Ok(())
    }

    /// Resolves generic trait-impl definitions after every linked trait
    /// declaration has been validated.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after linking every trait impl definition.
    ///
    /// # Errors
    ///
    /// Returns `ImplTraitResolution` when a trait declaration is missing or
    /// ambiguous.
    fn resolve_impl_definition_traits(&mut self) -> Result<(), RegistryError> {
        for definition in &self.impl_definitions {
            if definition.kind() != ImplKind::Trait {
                continue;
            }
            if let Some(declared) = definition.implemented_trait() {
                self.impl_definition_traits
                    .insert(definition.fragment_identity().clone(), declared);
                continue;
            }
            if let Some(trait_id) = definition.implemented_trait_id() {
                let Some(candidate) = self.traits_by_id.get(trait_id).copied() else {
                    return Err(RegistryError::impl_trait_resolution(
                        definition.fragment_identity().clone(),
                    ));
                };
                self.impl_definition_traits
                    .insert(definition.fragment_identity().clone(), candidate);
                continue;
            }
            return Err(RegistryError::impl_trait_resolution(
                definition.fragment_identity().clone(),
            ));
        }
        Ok(())
    }

    /// Freezes deterministic slices and hash indexes after successful
    /// validation.
    ///
    /// # Returns
    ///
    /// Returns the immutable registry snapshot.
    fn finish(mut self) -> ReflectRegistry {
        for implementations in self.impls_by_target.values_mut() {
            implementations.sort_by(|left, right| left.registry_cmp(right));
        }

        let types_by_type_name = group_types(&self.types, TypeDescriptor::type_name);
        let types_by_query_name = group_types(&self.types, TypeDescriptor::query_name);
        let definitions_by_rust_path = group_definitions(&self.definitions, TypeDefinitionDescriptor::rust_path);
        let definitions_by_query_name = group_definitions(&self.definitions, TypeDefinitionDescriptor::query_name);
        let mut trait_definitions: Vec<_> = self.traits_by_id.into_iter().collect();
        trait_definitions.sort_by(|(left_id, _), (right_id, _)| {
            self.trait_fragments
                .get(left_id)
                .cmp(&self.trait_fragments.get(right_id))
        });
        let traits_by_rust_path = group_traits(&trait_definitions);
        let type_fragments = self
            .types_by_id
            .iter()
            .map(|(type_id, (_, identity))| (*type_id, identity.clone()))
            .collect();
        let types_by_id = self
            .types_by_id
            .into_iter()
            .map(|(type_id, (descriptor, _))| (type_id, descriptor))
            .collect();
        let definition_fragments = self
            .definitions_by_id
            .iter()
            .map(|(id, (_, identity))| (*id, identity.clone()))
            .collect();
        let definitions_by_id = self
            .definitions_by_id
            .into_iter()
            .map(|(id, (descriptor, _))| (id, descriptor))
            .collect();
        let impls_by_target = self
            .impls_by_target
            .into_iter()
            .map(|(type_id, implementations)| (type_id, implementations.into_boxed_slice()))
            .collect::<HashMap<_, _>>();
        let effective_views_by_target = impls_by_target
            .iter()
            .map(|(type_id, implementations)| (*type_id, EffectiveTypeView::new(implementations)))
            .collect();
        let mut type_capability_descriptors: HashMap<TypeId, Vec<CapabilityDescriptor>> = HashMap::new();
        let mut definition_capability_descriptors: HashMap<TypeDefinitionId, Vec<CapabilityDescriptor>> =
            HashMap::new();
        let mut capability_fragments = HashMap::new();
        for ((target, capability_id), (descriptor, identity)) in self.capabilities {
            match target {
                CapabilityTarget::Type(type_id) => {
                    type_capability_descriptors.entry(type_id).or_default().push(descriptor);
                }
                CapabilityTarget::TypeDefinition(definition_id) => {
                    definition_capability_descriptors
                        .entry(definition_id)
                        .or_default()
                        .push(descriptor);
                }
            }
            capability_fragments.insert((target, capability_id), identity);
        }
        let capabilities_by_target = type_capability_descriptors
            .into_iter()
            .map(|(type_id, descriptors)| {
                let capabilities = TypeCapabilities::try_new(descriptors)
                    .expect("registry capability conflicts were validated before freezing");
                (type_id, capabilities)
            })
            .collect();
        let capabilities_by_definition = definition_capability_descriptors
            .into_iter()
            .map(|(definition_id, descriptors)| {
                let capabilities = TypeCapabilities::try_new(descriptors)
                    .expect("registry capability conflicts were validated before freezing");
                (definition_id, capabilities)
            })
            .collect();
        let indexes = RegistryIndexes {
            impl_definition_traits: self.impl_definition_traits,
            types_by_id,
            type_fragments,
            types_by_type_name,
            types_by_query_name,
            definitions_by_id,
            definition_fragments,
            definitions_by_rust_path,
            definitions_by_query_name,
            traits_by_id: trait_definitions.into_iter().collect(),
            traits_by_rust_path,
            impls_by_target,
            effective_views_by_target,
            capabilities_by_target,
            capabilities_by_definition,
            capability_fragments,
            capability_origins: self.capability_origins,
        };
        ReflectRegistry {
            types: self.types.into_boxed_slice(),
            definitions: self.definitions.into_boxed_slice(),
            impl_definitions: self.impl_definitions.into_boxed_slice(),
            indexes,
            empty_effective_view: EffectiveTypeView::empty(),
            empty_capabilities: TypeCapabilities::default(),
        }
    }
}

/// Groups generic declarations by one static name in fragment order.
fn group_definitions(
    definitions: &[&'static TypeDefinitionDescriptor],
    name: fn(&TypeDefinitionDescriptor) -> &'static str,
) -> HashMap<&'static str, Box<[&'static TypeDefinitionDescriptor]>> {
    let mut groups: HashMap<_, Vec<_>> = HashMap::new();
    for descriptor in definitions {
        groups.entry(name(descriptor)).or_default().push(*descriptor);
    }
    groups
        .into_iter()
        .map(|(key, descriptors)| (key, descriptors.into_boxed_slice()))
        .collect()
}

/// Builds an immutable registry from every linker-discovered registration fragment.
///
/// # Returns
///
/// Returns the fully validated registry snapshot.
///
/// # Errors
///
/// Returns the first conflict found while validating the discovered fragments.
pub(crate) fn build_inventory_registry() -> Result<ReflectRegistry, RegistryError> {
    build_registry_from_iter(inventory::iter::<RegistrationFragment>.into_iter())
}

/// Builds an immutable registry from an explicit static fragment slice.
///
/// # Parameters
///
/// - `fragments`: Static registration fragments to validate and include.
///
/// # Returns
///
/// Returns the fully validated registry snapshot.
///
/// # Errors
///
/// Returns the first conflict found while validating the supplied fragments.
pub(crate) fn build_registry(fragments: &[&'static RegistrationFragment]) -> Result<ReflectRegistry, RegistryError> {
    build_registry_from_iter(fragments.iter().copied())
}

/// Initializes a supplied cache from an explicit static fragment slice.
///
/// # Parameters
///
/// - `cache`: Process-wide cache that receives the registry or its validation error.
/// - `fragments`: Static registration fragments used for the one-time initialization.
///
/// # Returns
///
/// Returns a shared reference to the cached registry.
///
/// # Errors
///
/// Returns a clone of the validation error stored in the cache when initialization fails.
pub(crate) fn initialize_registry(
    cache: &'static OnceLock<Result<ReflectRegistry, RegistryError>>,
    fragments: &'static [&'static RegistrationFragment],
) -> Result<&'static ReflectRegistry, RegistryError> {
    initialize_cached(cache, || build_registry(fragments))
}

/// Returns the cached registry or a clone of its cached immutable error.
///
/// # Parameters
///
/// - `cache`: Process-wide cache to inspect or initialize.
/// - `initialize`: One-time initializer invoked only when the cache is empty.
///
/// # Returns
///
/// Returns a shared reference to the cached registry.
///
/// # Errors
///
/// Returns a clone of the immutable error stored by the initializer.
pub(super) fn initialize_cached(
    cache: &'static OnceLock<Result<ReflectRegistry, RegistryError>>,
    initialize: impl FnOnce() -> Result<ReflectRegistry, RegistryError>,
) -> Result<&'static ReflectRegistry, RegistryError> {
    match cache.get_or_init(initialize) {
        Ok(registry) => Ok(registry),
        Err(error) => Err(error.clone()),
    }
}

/// Sorts, materializes, fully validates, and freezes a fragment iterator.
///
/// # Parameters
///
/// - `fragments`: Static fragments to materialize in stable identity order.
///
/// # Returns
///
/// Returns the immutable registry snapshot built from the fragments.
///
/// # Errors
///
/// Returns an error when fragment identities or their materialized registrations conflict.
fn build_registry_from_iter(
    fragments: impl Iterator<Item = &'static RegistrationFragment>,
) -> Result<ReflectRegistry, RegistryError> {
    let mut pending: Vec<_> = fragments
        .map(|fragment| PendingFragment {
            fragment,
            identity: fragment.identity(),
        })
        .collect();
    pending.sort_by(|left, right| left.identity.cmp(&right.identity));
    validate_fragment_identities(&pending)?;

    let mut materialized = Vec::with_capacity(pending.len());
    for pending_fragment in pending {
        let payload = pending_fragment.fragment.build();
        materialized.push(MaterializedFragment {
            identity: pending_fragment.identity,
            declared_kind: pending_fragment.fragment.kind(),
            declared_target: pending_fragment.fragment.target_identity(),
            payload,
        });
    }

    validate_and_freeze_materialized(materialized)
}

/// Validates and freezes already materialized fragments through the common
/// post-factory registry path.
///
/// # Parameters
///
/// - `fragments`: Materialized registrations to validate and freeze.
///
/// # Returns
///
/// Returns the immutable registry snapshot.
///
/// # Errors
///
/// Returns an error when identities, declared payload metadata, or registered facts conflict.
pub(crate) fn validate_and_freeze_materialized(
    mut fragments: Vec<MaterializedFragment>,
) -> Result<ReflectRegistry, RegistryError> {
    fragments.sort_by(|left, right| left.identity.cmp(&right.identity));
    validate_identities(fragments.iter().map(|fragment| &fragment.identity))?;

    let mut builder = RegistryBuilder::default();
    for fragment in fragments {
        if fragment.declared_kind != fragment.payload.kind()
            || fragment.declared_target != fragment.payload.runtime_identity()
        {
            return Err(RegistryError::identity_conflict(
                fragment.identity.clone(),
                fragment.identity,
            ));
        }
        builder.push(BuiltFragment {
            identity: fragment.identity,
            payload: fragment.payload,
        })?;
    }
    builder.resolve_intrinsic_capabilities()?;
    builder.resolve_impl_definition_traits()?;
    Ok(builder.finish())
}

/// Detects exact duplicates and content changes before any payload is built.
///
/// # Parameters
///
/// - `fragments`: Pending fragments in stable identity order.
///
/// # Returns
///
/// Returns `Ok(())` when all source identities are unique and consistent.
///
/// # Errors
///
/// Returns an error when two fragments duplicate or conflict on a source identity.
fn validate_fragment_identities(fragments: &[PendingFragment]) -> Result<(), RegistryError> {
    validate_identities(fragments.iter().map(|fragment| &fragment.identity))
}

/// Validates sorted stable identities without inspecting their payloads.
///
/// # Parameters
///
/// - `identities`: Stable fragment identities in sorted order.
///
/// # Returns
///
/// Returns `Ok(())` when every identity is unique and consistent.
///
/// # Errors
///
/// Returns an error when adjacent identities are duplicates or conflict on their source.
fn validate_identities<'identity>(
    identities: impl IntoIterator<Item = &'identity FragmentIdentity>,
) -> Result<(), RegistryError> {
    let mut identities = identities.into_iter();
    let Some(mut left) = identities.next() else {
        return Ok(());
    };
    for right in identities {
        if left == right {
            return Err(RegistryError::duplicate_fragment(left.clone(), right.clone()));
        }
        if left.same_source_identity(right) {
            return Err(RegistryError::identity_conflict(left.clone(), right.clone()));
        }
        left = right;
    }
    Ok(())
}

/// Groups descriptors by one static name without leaking hash iteration order.
fn group_types(
    types: &[&'static TypeDescriptor],
    name: fn(&TypeDescriptor) -> &'static str,
) -> HashMap<&'static str, Box<[&'static TypeDescriptor]>> {
    let mut groups: HashMap<_, Vec<_>> = HashMap::new();
    for descriptor in types {
        groups.entry(name(descriptor)).or_default().push(*descriptor);
    }
    groups
        .into_iter()
        .map(|(key, descriptors)| (key, descriptors.into_boxed_slice()))
        .collect()
}

/// Groups trait definitions by their complete diagnostic paths in fragment
/// order.
fn group_traits(
    traits: &[(TraitId, &'static TraitDefinitionDescriptor)],
) -> HashMap<&'static str, Box<[&'static TraitDefinitionDescriptor]>> {
    let mut groups: HashMap<_, Vec<_>> = HashMap::new();
    for (_, descriptor) in traits {
        groups.entry(descriptor.rust_path()).or_default().push(*descriptor);
    }
    groups
        .into_iter()
        .map(|(path, descriptors)| (path, descriptors.into_boxed_slice()))
        .collect()
}
