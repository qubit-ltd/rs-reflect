// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
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
///
/// Type membership and capabilities are independent: adding capabilities does
/// not insert the target into [`ReflectRegistry::types`]. Pass the built
/// registry explicitly to both method lookup and invocation.
///
/// # Examples
///
/// ```
/// use qubit_reflect::TypeDescriptor;
/// use qubit_reflect::capability::{CapabilityDescriptor, CapabilityKey};
/// use qubit_reflect::identity::{CapabilityId, FragmentIdentity};
/// use qubit_reflect::registry::RegistrySnapshotBuilder;
///
/// let key = CapabilityKey::<u32>::new(CapabilityId::new("example.limit")?);
/// let target = TypeDescriptor::of::<u32>();
/// let mut builder = RegistrySnapshotBuilder::new();
/// builder.add_type_capabilities(target, vec![CapabilityDescriptor::with_adapter(key, 7)],
///     FragmentIdentity::new("example", "config", 1, 1, "capability", 1));
/// let registry = builder.build()?;
/// assert!(registry.types().is_empty());
/// assert_eq!(registry.capability(target, key)?, Some(&7));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct RegistrySnapshotBuilder {
    /// Owned facts awaiting transactional validation.
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
    ///
    /// Consumes this builder and publishes no partial result on failure. Empty
    /// input produces an empty snapshot. Intrinsic facts may be initialized
    /// during validation; providers must depend only on static type facts.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError`] for duplicate or changed source identities,
    /// conflicting targets, invalid trait links, or capability conflicts.
    /// Diagnostics retain the participating fragments and original conflicts.
    ///
    /// # Panics
    ///
    /// A panic from an intrinsic capability provider propagates; it is not
    /// converted into a registration error. Providers must not re-enter
    /// capability or registry initialization.
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
