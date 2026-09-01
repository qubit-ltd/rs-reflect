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

use crate::descriptor::AppliedTraitId;
use crate::descriptor::ImplDefinitionDescriptor;
use crate::descriptor::ImplDescriptor;
use crate::descriptor::ImplKind;
use crate::descriptor::TraitDefinitionDescriptor;
use crate::descriptor::TraitId;
use crate::descriptor::TypeDescriptor;
use crate::error::RegistryError;
use crate::identity::CapabilityId;
use crate::identity::ExternalTraitId;
use crate::identity::FragmentIdentity;
use crate::registry::EffectiveTypeView;
use crate::registry::fragment::FragmentPayload;
use crate::registry::fragment::RegistrationFragment;
use crate::registry::indexes::RegistryIndexes;
use crate::registry::internal::BuiltFragment;
use crate::registry::internal::MaterializedFragment;
use crate::registry::internal::PendingFragment;
use crate::registry::registry::ReflectRegistry;

/// Accumulates validated payloads without exposing partial registry state.
#[derive(Default)]
struct RegistryBuilder {
    types: Vec<&'static TypeDescriptor>,
    types_by_id: HashMap<TypeId, (&'static TypeDescriptor, FragmentIdentity)>,
    traits_by_id: HashMap<TraitId, &'static TraitDefinitionDescriptor>,
    trait_fragments: HashMap<TraitId, FragmentIdentity>,
    external_traits: HashMap<ExternalTraitId, (&'static TraitDefinitionDescriptor, FragmentIdentity)>,
    trait_impls: HashMap<(TypeId, AppliedTraitId), FragmentIdentity>,
    impl_definitions: Vec<&'static ImplDefinitionDescriptor>,
    impls_by_target: HashMap<TypeId, Vec<&'static ImplDescriptor>>,
    capabilities: HashMap<(TypeId, CapabilityId), (TypeId, FragmentIdentity)>,
    fragment_identities: Vec<FragmentIdentity>,
}

impl RegistryBuilder {
    /// Validates one materialized fragment and records its payload privately.
    fn push(&mut self, built: BuiltFragment) -> Result<(), RegistryError> {
        match built.payload {
            FragmentPayload::Type(descriptor) => self.push_type(descriptor, &built.identity)?,
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
                let key = (registration.target_type_id(), registration.descriptor().id().clone());
                if let Some((_, first)) = self.capabilities.get(&key) {
                    return Err(RegistryError::capability_conflict(first.clone(), built.identity));
                }
                self.capabilities
                    .insert(key, (registration.descriptor().adapter_type(), built.identity.clone()));
            }
        }
        self.fragment_identities.push(built.identity);
        Ok(())
    }

    /// Adds one unique concrete root descriptor.
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
        Ok(())
    }

    /// Adds one trait descriptor and audits reflected and external identities.
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
        self.trait_fragments.insert(definition_id.clone(), identity.clone());
        self.traits_by_id.entry(definition_id).or_insert(descriptor);
        Ok(())
    }

    /// Adds one impl after validating its outer, definition, and target
    /// identities.
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
    fn resolve_impl_definition_traits(&self) -> Result<(), RegistryError> {
        for definition in &self.impl_definitions {
            if definition.kind() != ImplKind::Trait || definition.implemented_trait().is_some() {
                continue;
            }
            if let Some(trait_id) = definition.implemented_trait_id() {
                let Some(candidate) = self.traits_by_id.get(trait_id).copied() else {
                    return Err(RegistryError::impl_trait_resolution(
                        definition.fragment_identity().clone(),
                    ));
                };
                if !definition.resolve_implemented_trait(candidate) {
                    return Err(RegistryError::impl_trait_resolution(
                        definition.fragment_identity().clone(),
                    ));
                }
                continue;
            }
            let Some(path) = definition.implemented_trait_path() else {
                return Err(RegistryError::impl_trait_resolution(
                    definition.fragment_identity().clone(),
                ));
            };
            let candidates: Vec<_> = self
                .traits_by_id
                .values()
                .copied()
                .filter(|descriptor| {
                    matches!(descriptor.trait_id(), TraitId::Reflected(_))
                        && reflected_trait_path_matches(path, descriptor.rust_path())
                })
                .collect();
            let candidate = match candidates.as_slice() {
                [candidate] => *candidate,
                [] => {
                    let compatible: Vec<_> = self
                        .traits_by_id
                        .values()
                        .copied()
                        .filter(|descriptor| {
                            matches!(descriptor.trait_id(), TraitId::Reflected(_))
                                && definition.matches_trait_definition(descriptor)
                        })
                        .collect();
                    let [candidate] = compatible.as_slice() else {
                        return Err(RegistryError::impl_trait_resolution(
                            definition.fragment_identity().clone(),
                        ));
                    };
                    *candidate
                }
                _ => {
                    return Err(RegistryError::impl_trait_resolution(
                        definition.fragment_identity().clone(),
                    ));
                }
            };
            if !definition.resolve_implemented_trait(candidate) {
                return Err(RegistryError::impl_trait_resolution(
                    definition.fragment_identity().clone(),
                ));
            }
        }
        Ok(())
    }

    /// Freezes deterministic slices and hash indexes after successful
    /// validation.
    fn finish(mut self) -> ReflectRegistry {
        for implementations in self.impls_by_target.values_mut() {
            implementations.sort_by(|left, right| left.registry_cmp(right));
        }

        let types_by_type_name = group_types(&self.types, TypeDescriptor::type_name);
        let types_by_query_name = group_types(&self.types, TypeDescriptor::query_name);
        let mut trait_definitions: Vec<_> = self.traits_by_id.into_iter().collect();
        trait_definitions.sort_by(|(left_id, _), (right_id, _)| {
            self.trait_fragments
                .get(left_id)
                .cmp(&self.trait_fragments.get(right_id))
        });
        let traits_by_rust_path = group_traits(&trait_definitions);
        let types_by_id = self
            .types_by_id
            .into_iter()
            .map(|(type_id, (descriptor, _))| (type_id, descriptor))
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
        let capability_fragments = self
            .capabilities
            .into_iter()
            .map(|(key, (_, identity))| (key, identity))
            .collect();
        let indexes = RegistryIndexes {
            types_by_id,
            types_by_type_name,
            types_by_query_name,
            traits_by_id: trait_definitions.into_iter().collect(),
            traits_by_rust_path,
            impls_by_target,
            effective_views_by_target,
            capability_fragments,
            fragment_identities: self.fragment_identities.into_boxed_slice(),
        };
        ReflectRegistry {
            types: self.types.into_boxed_slice(),
            impl_definitions: self.impl_definitions.into_boxed_slice(),
            indexes,
            empty_effective_view: EffectiveTypeView::empty(),
        }
    }
}

/// Returns whether a source trait path uniquely denotes a registered reflected
/// declaration after removing Rust-relative path anchors.
fn reflected_trait_path_matches(source_path: &str, registered_path: &str) -> bool {
    let mut relative_path = source_path;
    while let Some(stripped) = relative_path
        .strip_prefix("crate::")
        .or_else(|| relative_path.strip_prefix("self::"))
        .or_else(|| relative_path.strip_prefix("super::"))
    {
        relative_path = stripped;
    }
    registered_path == relative_path
        || registered_path
            .strip_suffix(relative_path)
            .is_some_and(|prefix| prefix.ends_with("::"))
}

/// Builds a registry from all linker-discovered fragments.
pub(crate) fn build_inventory_registry() -> Result<ReflectRegistry, RegistryError> {
    build_registry_from_iter(inventory::iter::<RegistrationFragment>.into_iter())
}

/// Builds a registry from an explicit static fragment slice.
pub(crate) fn build_registry(fragments: &[&'static RegistrationFragment]) -> Result<ReflectRegistry, RegistryError> {
    build_registry_from_iter(fragments.iter().copied())
}

/// Initializes a supplied cache from an explicit static fragment slice.
pub(crate) fn initialize_registry(
    cache: &'static OnceLock<Result<ReflectRegistry, RegistryError>>,
    fragments: &'static [&'static RegistrationFragment],
) -> Result<&'static ReflectRegistry, RegistryError> {
    initialize_cached(cache, || build_registry(fragments))
}

/// Returns the cached registry or a clone of its cached immutable error.
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
    builder.resolve_impl_definition_traits()?;
    Ok(builder.finish())
}

/// Detects exact duplicates and content changes before any payload is built.
fn validate_fragment_identities(fragments: &[PendingFragment]) -> Result<(), RegistryError> {
    validate_identities(fragments.iter().map(|fragment| &fragment.identity))
}

/// Validates sorted stable identities without inspecting their payloads.
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
