// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Private hash indexes behind the immutable public registry snapshot.

use std::any::TypeId;
use std::collections::HashMap;

use crate::capability::CapabilityOrigin;
use crate::capability::TypeCapabilities;
use crate::descriptor::ImplDescriptor;
use crate::descriptor::TraitDefinitionDescriptor;
use crate::descriptor::TraitId;
use crate::descriptor::TypeDefinitionDescriptor;
use crate::descriptor::TypeDefinitionId;
use crate::descriptor::TypeDescriptor;
use crate::identity::CapabilityId;
use crate::identity::FragmentIdentity;
use crate::registry::EffectiveTypeView;
use crate::registry::fragment::CapabilityTarget;

/// Immutable lookup indexes built only after all fragments validate.
#[derive(Debug)]
pub(super) struct RegistryIndexes {
    /// Trait declarations resolved for each symbolic impl definition.
    pub(super) impl_definition_traits: HashMap<FragmentIdentity, &'static TraitDefinitionDescriptor>,
    /// Reflected roots keyed by exact process-local type identity.
    pub(super) types_by_id: HashMap<TypeId, &'static TypeDescriptor>,
    /// Source fragment associated with each registered root.
    pub(super) type_fragments: HashMap<TypeId, FragmentIdentity>,
    /// Roots grouped by compiler-provided type name.
    pub(super) types_by_type_name: HashMap<&'static str, Box<[&'static TypeDescriptor]>>,
    /// Roots grouped by reflection query name.
    pub(super) types_by_query_name: HashMap<&'static str, Box<[&'static TypeDescriptor]>>,
    /// Generic declarations keyed by process-local declaration identity.
    pub(super) definitions_by_id: HashMap<TypeDefinitionId, &'static TypeDefinitionDescriptor>,
    /// Source fragment associated with each generic declaration.
    pub(super) definition_fragments: HashMap<TypeDefinitionId, FragmentIdentity>,
    /// Generic declarations grouped by Rust source path.
    pub(super) definitions_by_rust_path: HashMap<&'static str, Box<[&'static TypeDefinitionDescriptor]>>,
    /// Generic declarations grouped by reflection query name.
    pub(super) definitions_by_query_name: HashMap<&'static str, Box<[&'static TypeDefinitionDescriptor]>>,
    /// Trait declarations keyed by reflected or external identity.
    #[allow(dead_code, reason = "consumed by the T21 effective-view implementation")]
    pub(super) traits_by_id: HashMap<TraitId, &'static TraitDefinitionDescriptor>,
    /// Trait declarations grouped by source path.
    pub(super) traits_by_rust_path: HashMap<&'static str, Box<[&'static TraitDefinitionDescriptor]>>,
    /// Concrete implementations grouped by target type identity.
    #[allow(dead_code, reason = "consumed by the T21 effective-view implementation")]
    pub(super) impls_by_target: HashMap<TypeId, Box<[&'static ImplDescriptor]>>,
    /// Frozen effective method view for each registered target type.
    pub(super) effective_views_by_target: HashMap<TypeId, EffectiveTypeView>,
    /// Effective capabilities keyed by concrete target identity.
    pub(super) capabilities_by_target: HashMap<TypeId, TypeCapabilities>,
    /// Effective capabilities keyed by generic declaration identity.
    pub(super) capabilities_by_definition: HashMap<TypeDefinitionId, TypeCapabilities>,
    /// Source fragments retained for capability conflict auditing.
    #[allow(dead_code, reason = "retained for registry conflict auditing")]
    pub(super) capability_fragments: HashMap<(CapabilityTarget, CapabilityId), FragmentIdentity>,
    /// Effective semantic origin for each registered capability.
    pub(super) capability_origins: HashMap<(CapabilityTarget, CapabilityId), CapabilityOrigin>,
    /// All unique fragments in deterministic order.
    #[allow(dead_code, reason = "retained for registry conflict auditing")]
    pub(super) fragment_identities: Box<[FragmentIdentity]>,
}
