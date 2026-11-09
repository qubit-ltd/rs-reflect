// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::sync::Arc;

use super::capabilities::Capabilities;
use super::capability::CapabilityDescriptor;
use super::capability::CapabilitySpec;
use super::capability_error::CapabilityError;
use super::capability_target::CapabilityTarget;
use crate::runtime::context::ReflectionContext;

/// A lightweight capability view bound to the context that performed lookup.
pub struct CapabilitiesView<'ctx> {
    /// Immutable capability records resolved for the target.
    pub(super) set: Arc<Capabilities>,
    /// Entity whose registrations populate this view.
    pub(super) target: CapabilityTarget<'ctx>,
    /// Context used to resolve descriptors returned by this view.
    pub(super) context: &'ctx ReflectionContext,
}

impl<'ctx> CapabilitiesView<'ctx> {
    /// Creates a view over resolved capability records.
    ///
    /// # Parameters
    ///
    /// * `set` — immutable records associated with the target.
    /// * `target` — entity whose capabilities are described.
    /// * `context` — context used by returned descriptors.
    pub(crate) fn new(
        set: Arc<Capabilities>,
        target: CapabilityTarget<'ctx>,
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self { set, target, context }
    }

    /// Creates an empty view for a target that has no capability records.
    ///
    /// # Parameters
    ///
    /// * `target` — entity whose capability lookup returned no records.
    /// * `context` — context used by descriptors obtained from the view.
    pub(crate) fn empty(target: CapabilityTarget<'ctx>, context: &'ctx ReflectionContext) -> Self {
        Self::new(Arc::new(Capabilities::empty(target.id())), target, context)
    }

    /// Returns the number of capabilities available for this target.
    pub fn len(&self) -> usize {
        self.set.records.len()
    }

    /// Returns `true` when this target has no registered capabilities.
    pub fn is_empty(&self) -> bool {
        self.set.records.is_empty()
    }

    /// Returns the entity to which this view is bound.
    pub fn target(&self) -> CapabilityTarget<'ctx> {
        self.target
    }

    /// Finds a capability descriptor by its stable textual ID.
    ///
    /// Returns `None` when no capability with `id` is registered for this
    /// target.
    ///
    /// # Parameters
    ///
    /// * `id` — stable textual capability identifier.
    pub fn get(&self, id: &str) -> Option<CapabilityDescriptor<'ctx>> {
        self.set
            .records
            .iter()
            .position(|record| record.id.as_str() == id)
            .map(move |index| CapabilityDescriptor {
                set: Arc::clone(&self.set),
                index,
                target: self.target,
                context: self.context,
            })
    }

    /// Iterates over all capability descriptors for this target.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = CapabilityDescriptor<'ctx>> + '_ {
        self.set
            .records
            .iter()
            .enumerate()
            .map(move |(index, _record)| CapabilityDescriptor {
                set: Arc::clone(&self.set),
                index,
                target: self.target,
                context: self.context,
            })
    }

    /// Restores a capability descriptor when its adapter satisfies `C`.
    ///
    /// # Type Parameters
    ///
    /// * `C` — capability contract to verify.
    pub(crate) fn find<C: CapabilitySpec>(
        &self,
    ) -> Result<Option<super::capability::Capability<'ctx, C>>, CapabilityError> {
        let Some(descriptor) = self.get(C::ID) else {
            return Ok(None);
        };
        let registration_origin = descriptor.record().origin.clone();
        descriptor.downcast::<C>().map(Some).map_err(|error| match error {
            super::capability_error::CapabilityContractError::AdapterTypeMismatch {
                capability_id,
                expected,
                actual,
            } => CapabilityError::ContractMismatch {
                target: self.set.target.clone(),
                capability_id,
                expected_adapter: expected,
                actual_adapter: actual,
                origin: registration_origin,
            },
            other => CapabilityError::Contract(other),
        })
    }

    /// Restores a required capability or reports its absence or invalid
    /// contract.
    ///
    /// # Type Parameters
    ///
    /// * `C` — required capability contract.
    pub(crate) fn capability<C: CapabilitySpec>(
        &self,
    ) -> Result<super::capability::Capability<'ctx, C>, CapabilityError> {
        self.find::<C>()?.ok_or_else(|| CapabilityError::Missing {
            target: self.set.target.clone(),
            capability_id: C::ID,
        })
    }
}
