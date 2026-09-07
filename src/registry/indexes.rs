// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Private hash indexes behind the immutable public registry snapshot.

use std::any::TypeId;
use std::collections::HashMap;

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
    pub(super) impl_definition_traits: HashMap<FragmentIdentity, &'static TraitDefinitionDescriptor>,
    pub(super) types_by_id: HashMap<TypeId, &'static TypeDescriptor>,
    pub(super) type_fragments: HashMap<TypeId, FragmentIdentity>,
    pub(super) types_by_type_name: HashMap<&'static str, Box<[&'static TypeDescriptor]>>,
    pub(super) types_by_query_name: HashMap<&'static str, Box<[&'static TypeDescriptor]>>,
    pub(super) definitions_by_id: HashMap<TypeDefinitionId, &'static TypeDefinitionDescriptor>,
    pub(super) definition_fragments: HashMap<TypeDefinitionId, FragmentIdentity>,
    pub(super) definitions_by_rust_path: HashMap<&'static str, Box<[&'static TypeDefinitionDescriptor]>>,
    pub(super) definitions_by_query_name: HashMap<&'static str, Box<[&'static TypeDefinitionDescriptor]>>,
    #[allow(dead_code, reason = "consumed by the T21 effective-view implementation")]
    pub(super) traits_by_id: HashMap<TraitId, &'static TraitDefinitionDescriptor>,
    pub(super) traits_by_rust_path: HashMap<&'static str, Box<[&'static TraitDefinitionDescriptor]>>,
    #[allow(dead_code, reason = "consumed by the T21 effective-view implementation")]
    pub(super) impls_by_target: HashMap<TypeId, Box<[&'static ImplDescriptor]>>,
    pub(super) effective_views_by_target: HashMap<TypeId, EffectiveTypeView>,
    pub(super) capabilities_by_target: HashMap<TypeId, TypeCapabilities>,
    pub(super) capabilities_by_definition: HashMap<TypeDefinitionId, TypeCapabilities>,
    #[allow(dead_code, reason = "retained for registry conflict auditing")]
    pub(super) capability_fragments: HashMap<(CapabilityTarget, CapabilityId), FragmentIdentity>,
    #[allow(dead_code, reason = "retained for registry conflict auditing")]
    pub(super) fragment_identities: Box<[FragmentIdentity]>,
}
