// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::TypeId;
use std::sync::Arc;

use super::capabilities::Capabilities;
use super::capability_error::CapabilityContractError;
use super::capability_origin::CapabilityOrigin;
use super::capability_record::CapabilityRecord;
use super::capability_target::CapabilityTarget;
use super::context_registration_origin::ContextRegistrationOrigin;
use crate::runtime::context::ReflectionContext;

/// Binds a stable capability ID to exactly one Rust adapter contract.
pub trait CapabilitySpec: 'static {
    /// Concrete adapter type associated with this stable capability ID.
    type Adapter: Send + Sync + 'static;

    /// Stable textual identifier used to find this capability.
    const ID: &'static str;
}

/// A context-bound descriptor used to inspect and downcast a capability.
#[derive(Clone)]
pub struct CapabilityDescriptor<'ctx> {
    /// Immutable target capability registrations.
    pub(super) set: Arc<Capabilities>,
    /// Position of this capability's record in the shared set.
    pub(super) index: usize,
    /// Reflected entity to which the capability is bound.
    pub(super) target: CapabilityTarget<'ctx>,
    /// Context used to resolve registration provenance.
    pub(super) context: &'ctx ReflectionContext,
}

impl<'ctx> CapabilityDescriptor<'ctx> {
    /// Returns the stable textual identifier of this capability.
    pub fn id(&self) -> &'static str {
        self.record().id_str
    }

    /// Returns the concrete Rust type identity recorded for the adapter.
    pub fn adapter_type_id(&self) -> TypeId {
        self.record().adapter_type
    }

    /// Returns the entity to which this descriptor is bound.
    pub fn target(&self) -> CapabilityTarget<'ctx> {
        self.target
    }

    /// Returns the source that registered this capability in the context.
    pub fn origin(&self) -> CapabilityOrigin<'ctx> {
        bind_origin(&self.record().origin, self.context)
    }

    /// Converts this descriptor to the requested typed capability contract.
    ///
    /// Returns an error when either the stable ID or adapter type differs from
    /// `C`.
    ///
    /// # Type Parameters
    /// - `C`: capability contract whose ID and adapter type must match.
    pub fn downcast<C: CapabilitySpec>(self) -> Result<Capability<'ctx, C>, CapabilityContractError> {
        let record = self.record();
        if record.id.as_str() != C::ID {
            return Err(CapabilityContractError::IdMismatch {
                expected: C::ID,
                actual: record.id,
            });
        }
        let expected = TypeId::of::<C::Adapter>();
        if record.adapter_type != expected {
            return Err(CapabilityContractError::AdapterTypeMismatch {
                capability_id: record.id,
                expected,
                actual: record.adapter_type,
            });
        }
        let adapter = match record.adapter.clone().downcast::<C::Adapter>() {
            Ok(adapter) => adapter,
            Err(_) => {
                return Err(CapabilityContractError::CorruptAdapter {
                    capability_id: record.id,
                    expected,
                });
            }
        };
        Ok(Capability {
            target: self.target,
            adapter,
            origin: bind_origin(&record.origin, self.context),
        })
    }

    /// Returns the registered record for this descriptor.
    pub(super) fn record(&self) -> &CapabilityRecord {
        &self.set.records[self.index]
    }
}

/// A capability handle whose adapter and reflection context are fixed by
/// lookup.
///
/// # Type Parameters
/// - `C`: stable capability contract implemented by the adapter.
pub struct Capability<'ctx, C: CapabilitySpec> {
    /// Entity to which the capability is bound.
    target: CapabilityTarget<'ctx>,
    /// Adapter implementing the typed capability contract.
    adapter: Arc<C::Adapter>,
    /// Registration provenance bound to the lookup context.
    origin: CapabilityOrigin<'ctx>,
}

impl<'ctx, C: CapabilitySpec> Capability<'ctx, C> {
    /// Returns the entity to which this capability is bound.
    pub fn target(&self) -> CapabilityTarget<'ctx> {
        self.target
    }

    /// Returns the adapter implementing the capability contract.
    pub fn adapter(&self) -> &C::Adapter {
        &self.adapter
    }

    /// Returns the source that registered this capability in the context.
    pub fn origin(&self) -> CapabilityOrigin<'ctx> {
        self.origin.clone()
    }
}

impl<C: CapabilitySpec> Clone for Capability<'_, C> {
    /// Clones the handle while sharing its adapter allocation.
    fn clone(&self) -> Self {
        Self {
            target: self.target,
            adapter: self.adapter.clone(),
            origin: self.origin.clone(),
        }
    }
}

/// Binds stored registration provenance to the context that performed lookup.
///
/// # Parameters
/// - `origin`: registration provenance stored in the immutable capability set.
/// - `context`: reflection context that owns the descriptor.
fn bind_origin<'ctx>(
    origin: &super::capability_registration_origin::CapabilityRegistrationOrigin,
    context: &'ctx ReflectionContext,
) -> CapabilityOrigin<'ctx> {
    match origin {
        super::capability_registration_origin::CapabilityRegistrationOrigin::Linked(registration) => {
            CapabilityOrigin::Linked {
                registration: ContextRegistrationOrigin::new(registration.clone(), context),
            }
        }
        super::capability_registration_origin::CapabilityRegistrationOrigin::Explicit(registration) => {
            CapabilityOrigin::Explicit {
                registration: ContextRegistrationOrigin::new(registration.clone(), context),
            }
        }
    }
}
