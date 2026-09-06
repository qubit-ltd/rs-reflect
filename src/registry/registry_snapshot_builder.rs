// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

//! Explicit construction of isolated reflection registry snapshots.

use crate::capability::CapabilityDescriptor;
use crate::descriptor::ImplDefinitionDescriptor;
use crate::descriptor::ImplDescriptor;
use crate::descriptor::TraitDefinitionDescriptor;
use crate::descriptor::TypeDefinitionDescriptor;
use crate::descriptor::TypeDescriptor;
use crate::error::RegistryError;
use crate::identity::FragmentIdentity;
use crate::registry::fragment::CapabilityRegistration;
use crate::registry::fragment::FragmentPayload;
use crate::registry::internal::MaterializedFragment;
use crate::registry::registry::ReflectRegistry;

/// Collects typed reflection facts for one isolated registry snapshot.
///
/// The builder does not read linker inventory or mutate the process-wide
/// registry. Validation happens transactionally when [`Self::build`] consumes
/// the collected facts.
pub struct RegistrySnapshotBuilder {
    fragments: Vec<MaterializedFragment>,
}

impl RegistrySnapshotBuilder {
    /// Creates an empty snapshot builder.
    #[must_use]
    pub const fn new() -> Self {
        Self { fragments: Vec::new() }
    }

    /// Adds one concrete reflected type as a snapshot member.
    pub fn add_type(&mut self, descriptor: &'static TypeDescriptor, source: FragmentIdentity) -> &mut Self {
        self.push(FragmentPayload::Type(descriptor), source)
    }

    /// Adds one source-level generic type declaration as a snapshot member.
    pub fn add_definition(
        &mut self,
        descriptor: &'static TypeDefinitionDescriptor,
        source: FragmentIdentity,
    ) -> &mut Self {
        self.push(FragmentPayload::TypeDefinition(descriptor), source)
    }

    /// Adds one reflected or external trait declaration.
    pub fn add_trait(&mut self, descriptor: &'static TraitDefinitionDescriptor, source: FragmentIdentity) -> &mut Self {
        self.push(FragmentPayload::Trait(descriptor), source)
    }

    /// Adds one generic, blanket, constrained, or concrete impl declaration.
    pub fn add_impl_definition(
        &mut self,
        descriptor: &'static ImplDefinitionDescriptor,
        source: FragmentIdentity,
    ) -> &mut Self {
        self.push(FragmentPayload::ImplDefinition(descriptor), source)
    }

    /// Adds one concrete impl application without implicitly adding its type.
    pub fn add_impl(&mut self, descriptor: &'static ImplDescriptor, source: FragmentIdentity) -> &mut Self {
        self.push(FragmentPayload::Impl(descriptor), source)
    }

    /// Adds capabilities for a concrete type without adding the type as a
    /// snapshot member.
    pub fn add_type_capabilities(
        &mut self,
        target: &'static TypeDescriptor,
        capabilities: Vec<CapabilityDescriptor>,
        source: FragmentIdentity,
    ) -> &mut Self {
        self.push(
            FragmentPayload::Capability(CapabilityRegistration::for_type(target, capabilities)),
            source,
        )
    }

    /// Adds capabilities for a generic declaration without adding the
    /// declaration as a snapshot member.
    pub fn add_definition_capabilities(
        &mut self,
        target: &'static TypeDefinitionDescriptor,
        capabilities: Vec<CapabilityDescriptor>,
        source: FragmentIdentity,
    ) -> &mut Self {
        self.push(
            FragmentPayload::Capability(CapabilityRegistration::for_definition(target, capabilities)),
            source,
        )
    }

    /// Validates every collected fact and freezes an isolated registry.
    pub fn build(self) -> Result<ReflectRegistry, RegistryError> {
        super::registry_builder::validate_and_freeze_materialized(self.fragments)
    }

    /// Records one typed payload together with its derived validation facts.
    fn push(&mut self, payload: FragmentPayload, source: FragmentIdentity) -> &mut Self {
        let declared_kind = payload.kind();
        let declared_target = payload.runtime_identity();
        self.fragments.push(MaterializedFragment {
            identity: source,
            declared_kind,
            declared_target,
            payload,
        });
        self
    }
}

impl Default for RegistrySnapshotBuilder {
    fn default() -> Self {
        Self::new()
    }
}
