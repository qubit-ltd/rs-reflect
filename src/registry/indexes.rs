// qubit-style: allow public-type-layout
//! Private hash indexes behind the immutable public registry snapshot.

use std::any::TypeId;
use std::collections::HashMap;

use crate::descriptor::ImplDescriptor;
use crate::descriptor::TraitDefinitionDescriptor;
use crate::descriptor::TraitId;
use crate::descriptor::TypeDescriptor;
use crate::identity::CapabilityId;
use crate::identity::FragmentIdentity;

/// Immutable lookup indexes built only after all fragments validate.
#[derive(Debug)]
pub(super) struct RegistryIndexes {
    pub(super) types_by_id: HashMap<TypeId, &'static TypeDescriptor>,
    pub(super) types_by_type_name: HashMap<&'static str, Box<[&'static TypeDescriptor]>>,
    pub(super) types_by_query_name: HashMap<&'static str, Box<[&'static TypeDescriptor]>>,
    #[allow(dead_code, reason = "consumed by the T21 effective-view implementation")]
    pub(super) traits_by_id: HashMap<TraitId, &'static TraitDefinitionDescriptor>,
    pub(super) traits_by_rust_path: HashMap<&'static str, Box<[&'static TraitDefinitionDescriptor]>>,
    #[allow(dead_code, reason = "consumed by the T21 effective-view implementation")]
    pub(super) impls_by_target: HashMap<TypeId, Box<[&'static ImplDescriptor]>>,
    #[allow(dead_code, reason = "retained for registry conflict auditing")]
    pub(super) capability_fragments: HashMap<(TypeId, CapabilityId), FragmentIdentity>,
    #[allow(dead_code, reason = "retained for registry conflict auditing")]
    pub(super) fragment_identities: Box<[FragmentIdentity]>,
}
