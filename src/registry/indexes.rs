//! Private hash indexes behind the immutable public registry snapshot.

use std::any::TypeId;
use std::collections::HashMap;

use crate::descriptor::{ImplDescriptor, TraitDefinitionDescriptor, TraitId, TypeDescriptor};
use crate::identity::{CapabilityId, FragmentIdentity};

/// Immutable lookup indexes built only after all fragments validate.
#[derive(Debug)]
pub(super) struct RegistryIndexes {
    pub(super) types_by_id: HashMap<TypeId, &'static TypeDescriptor>,
    pub(super) types_by_type_name: HashMap<&'static str, Box<[&'static TypeDescriptor]>>,
    pub(super) types_by_query_name: HashMap<&'static str, Box<[&'static TypeDescriptor]>>,
    #[allow(
        dead_code,
        reason = "consumed by the T21 effective-view implementation"
    )]
    pub(super) traits_by_id: HashMap<TraitId, &'static TraitDefinitionDescriptor>,
    #[allow(
        dead_code,
        reason = "consumed by the T21 effective-view implementation"
    )]
    pub(super) impls_by_target: HashMap<TypeId, Box<[&'static ImplDescriptor]>>,
    #[allow(dead_code, reason = "retained for registry conflict auditing")]
    pub(super) capability_fragments: HashMap<(TypeId, CapabilityId), FragmentIdentity>,
    #[allow(dead_code, reason = "retained for registry conflict auditing")]
    pub(super) fragment_identities: Box<[FragmentIdentity]>,
}

impl RegistryIndexes {
    /// Creates the complete immutable set of validated registry indexes.
    pub(super) fn new(
        types_by_id: HashMap<TypeId, &'static TypeDescriptor>,
        types_by_type_name: HashMap<&'static str, Box<[&'static TypeDescriptor]>>,
        types_by_query_name: HashMap<&'static str, Box<[&'static TypeDescriptor]>>,
        traits_by_id: HashMap<TraitId, &'static TraitDefinitionDescriptor>,
        impls_by_target: HashMap<TypeId, Box<[&'static ImplDescriptor]>>,
        capability_fragments: HashMap<(TypeId, CapabilityId), FragmentIdentity>,
        fragment_identities: Box<[FragmentIdentity]>,
    ) -> Self {
        Self {
            types_by_id,
            types_by_type_name,
            types_by_query_name,
            traits_by_id,
            impls_by_target,
            capability_fragments,
            fragment_identities,
        }
    }
}
