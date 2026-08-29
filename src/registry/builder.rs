//! Transactional validation and construction of frozen registry snapshots.

use std::any::TypeId;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::OnceLock;

use crate::descriptor::{
    AppliedTraitId, ImplDescriptor, ImplKind, TraitDefinitionDescriptor, TraitId, TypeDescriptor,
};
use crate::error::RegistryError;
use crate::identity::{CapabilityId, ExternalTraitId, FragmentIdentity};
use crate::registry::fragment::{FragmentPayload, RegistrationFragment};
use crate::registry::indexes::RegistryIndexes;
use crate::registry::registry::ReflectRegistry;

/// A sorted fragment paired with its already materialized stable identity.
struct PendingFragment {
    fragment: &'static RegistrationFragment,
    identity: FragmentIdentity,
}

/// A built fragment retained until every cross-fragment check succeeds.
struct BuiltFragment {
    identity: FragmentIdentity,
    payload: FragmentPayload,
}

/// Accumulates validated payloads without exposing partial registry state.
#[derive(Default)]
struct RegistryBuilder {
    types: Vec<&'static TypeDescriptor>,
    types_by_id: HashMap<TypeId, (&'static TypeDescriptor, FragmentIdentity)>,
    traits_by_id: HashMap<TraitId, &'static TraitDefinitionDescriptor>,
    trait_fragments: HashMap<TraitId, FragmentIdentity>,
    external_traits:
        HashMap<ExternalTraitId, (&'static TraitDefinitionDescriptor, FragmentIdentity)>,
    trait_impls: HashMap<(TypeId, AppliedTraitId), FragmentIdentity>,
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
            FragmentPayload::Impl(descriptor) => self.push_impl(descriptor, &built.identity)?,
            FragmentPayload::Capability(registration) => {
                let key = (
                    registration.target_type_id(),
                    registration.descriptor().id().clone(),
                );
                if let Some((_, first)) = self.capabilities.get(&key) {
                    return Err(RegistryError::capability_conflict(
                        first.clone(),
                        built.identity,
                    ));
                }
                self.capabilities.insert(
                    key,
                    (
                        registration.descriptor().adapter_type(),
                        built.identity.clone(),
                    ),
                );
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
            return Err(RegistryError::identity_conflict(
                first.clone(),
                identity.clone(),
            ));
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
            if let Some((first_descriptor, first_identity)) = self.external_traits.get(external_id)
            {
                if !compatible_external_traits(first_descriptor, descriptor) {
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
            return Err(RegistryError::identity_conflict(
                first.clone(),
                identity.clone(),
            ));
        }
        self.trait_fragments
            .insert(definition_id.clone(), identity.clone());
        self.traits_by_id.entry(definition_id).or_insert(descriptor);
        Ok(())
    }

    /// Adds one impl after validating its outer, definition, and target identities.
    fn push_impl(
        &mut self,
        descriptor: &'static ImplDescriptor,
        identity: &FragmentIdentity,
    ) -> Result<(), RegistryError> {
        let definition_identity = descriptor.definition().fragment_identity();
        if definition_identity != identity {
            return Err(RegistryError::identity_conflict(
                definition_identity.clone(),
                identity.clone(),
            ));
        }

        let target_type_id = descriptor.target_type().type_id();
        if let Some(implemented_trait) = descriptor.implemented_trait() {
            let key = (target_type_id, implemented_trait.trait_id().clone());
            if let Some(first) = self.trait_impls.get(&key) {
                return Err(RegistryError::identity_conflict(
                    first.clone(),
                    identity.clone(),
                ));
            }
            self.trait_impls.insert(key, identity.clone());
        }
        self.impls_by_target
            .entry(target_type_id)
            .or_default()
            .push(descriptor);
        Ok(())
    }

    /// Freezes deterministic slices and hash indexes after successful validation.
    fn finish(mut self) -> ReflectRegistry {
        for implementations in self.impls_by_target.values_mut() {
            implementations.sort_by(compare_impls);
        }

        let types_by_type_name = group_types(&self.types, TypeDescriptor::type_name);
        let types_by_query_name = group_types(&self.types, TypeDescriptor::query_name);
        let types_by_id = self
            .types_by_id
            .into_iter()
            .map(|(type_id, (descriptor, _))| (type_id, descriptor))
            .collect();
        let impls_by_target = self
            .impls_by_target
            .into_iter()
            .map(|(type_id, implementations)| (type_id, implementations.into_boxed_slice()))
            .collect();
        let capability_fragments = self
            .capabilities
            .into_iter()
            .map(|(key, (_, identity))| (key, identity))
            .collect();
        let indexes = RegistryIndexes::new(
            types_by_id,
            types_by_type_name,
            types_by_query_name,
            self.traits_by_id,
            impls_by_target,
            capability_fragments,
            self.fragment_identities.into_boxed_slice(),
        );
        ReflectRegistry {
            types: self.types.into_boxed_slice(),
            indexes,
        }
    }
}

/// Builds a registry from all linker-discovered fragments.
pub(crate) fn build_inventory_registry() -> Result<ReflectRegistry, RegistryError> {
    build_registry_from_iter(inventory::iter::<RegistrationFragment>.into_iter())
}

/// Builds a registry from an explicit static fragment slice.
pub(crate) fn build_registry(
    fragments: &[&'static RegistrationFragment],
) -> Result<ReflectRegistry, RegistryError> {
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

    let mut built = Vec::with_capacity(pending.len());
    for pending_fragment in pending {
        let payload = pending_fragment.fragment.build();
        if pending_fragment.fragment.kind() != payload.kind()
            || pending_fragment.fragment.target_identity() != payload.runtime_identity()
        {
            return Err(RegistryError::identity_conflict(
                pending_fragment.identity.clone(),
                pending_fragment.identity,
            ));
        }
        built.push(BuiltFragment {
            identity: pending_fragment.identity,
            payload,
        });
    }

    let mut builder = RegistryBuilder::default();
    for fragment in built {
        builder.push(fragment)?;
    }
    Ok(builder.finish())
}

/// Detects exact duplicates and content changes before any payload is built.
fn validate_fragment_identities(fragments: &[PendingFragment]) -> Result<(), RegistryError> {
    for pair in fragments.windows(2) {
        let [left, right] = pair else {
            continue;
        };
        if left.identity == right.identity {
            return Err(RegistryError::duplicate_fragment(
                left.identity.clone(),
                right.identity.clone(),
            ));
        }
        if same_source_identity(&left.identity, &right.identity) {
            return Err(RegistryError::identity_conflict(
                left.identity.clone(),
                right.identity.clone(),
            ));
        }
    }
    Ok(())
}

/// Compares stable source facts while deliberately excluding content fingerprint.
fn same_source_identity(left: &FragmentIdentity, right: &FragmentIdentity) -> bool {
    left.declaring_crate() == right.declaring_crate()
        && left.module_path() == right.module_path()
        && left.line() == right.line()
        && left.column() == right.column()
        && left.member_kind() == right.member_kind()
}

/// Returns whether two aliases contribute mergeable facts for one external ID.
fn compatible_external_traits(
    left: &TraitDefinitionDescriptor,
    right: &TraitDefinitionDescriptor,
) -> bool {
    left.completeness() == right.completeness()
        && left.generic_definition() == right.generic_definition()
}

/// Groups descriptors by one static name without leaking hash iteration order.
fn group_types(
    types: &[&'static TypeDescriptor],
    name: fn(&TypeDescriptor) -> &'static str,
) -> HashMap<&'static str, Box<[&'static TypeDescriptor]>> {
    let mut groups: HashMap<_, Vec<_>> = HashMap::new();
    for descriptor in types {
        groups
            .entry(name(descriptor))
            .or_default()
            .push(*descriptor);
    }
    groups
        .into_iter()
        .map(|(key, descriptors)| (key, descriptors.into_boxed_slice()))
        .collect()
}

/// Orders concrete implementations by namespace and stable source identity.
fn compare_impls(left: &&'static ImplDescriptor, right: &&'static ImplDescriptor) -> Ordering {
    impl_kind_rank(left.kind())
        .cmp(&impl_kind_rank(right.kind()))
        .then_with(|| compare_impl_namespaces(left, right))
        .then_with(|| {
            left.definition()
                .fragment_identity()
                .cmp(right.definition().fragment_identity())
        })
}

/// Returns the deterministic inherent-before-trait rank.
const fn impl_kind_rank(kind: ImplKind) -> u8 {
    match kind {
        ImplKind::Inherent => 0,
        ImplKind::Trait => 1,
    }
}

/// Orders reflected impls by path and external impls by stable external ID.
fn compare_impl_namespaces(left: &ImplDescriptor, right: &ImplDescriptor) -> Ordering {
    let left_trait = left.implemented_trait();
    let right_trait = right.implemented_trait();
    match (left_trait, right_trait) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
        (Some(left_trait), Some(right_trait)) => match (
            left_trait.definition().trait_id(),
            right_trait.definition().trait_id(),
        ) {
            (TraitId::Reflected(_), TraitId::Reflected(_)) => {
                left_trait.rust_path().cmp(right_trait.rust_path())
            }
            (TraitId::External(left_id), TraitId::External(right_id)) => left_id.cmp(right_id),
            (TraitId::Reflected(_), TraitId::External(_)) => Ordering::Less,
            (TraitId::External(_), TraitId::Reflected(_)) => Ordering::Greater,
        },
    }
}
