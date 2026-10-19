use std::any::TypeId;

use super::capabilities::CapabilityRecord;
use super::capability_error::CapabilityContractError;
use super::capability_id::CapabilityIdentifier;
use super::capability_origin::CapabilityOrigin;
use super::capability_target::CapabilityTargetId;

/// Binds a stable capability ID to exactly one Rust adapter contract.
pub(crate) trait CapabilitySpec: 'static {
    type Adapter: Send + Sync + 'static;

    const ID: &'static str;
}

/// A context-independent descriptor for finding a typed adapter.
#[derive(Clone, Copy)]
pub(crate) struct CapabilityDescriptor<'ctx> {
    pub(super) record: &'ctx CapabilityRecord,
}

impl<'ctx> CapabilityDescriptor<'ctx> {
    pub(crate) fn id(self) -> CapabilityIdentifier {
        self.record.id
    }

    pub(crate) fn adapter_type_id(self) -> TypeId {
        self.record.adapter_type
    }

    pub(crate) fn target(self) -> &'ctx CapabilityTargetId {
        &self.record.target
    }

    pub(crate) fn origin(self) -> &'ctx CapabilityOrigin {
        &self.record.origin
    }

    pub(crate) fn downcast<C: CapabilitySpec>(
        self,
    ) -> Result<Capability<'ctx, C>, CapabilityContractError> {
        if self.record.id.as_str() != C::ID {
            return Err(CapabilityContractError::IdMismatch {
                expected: C::ID,
                actual: self.record.id,
            });
        }
        let expected = TypeId::of::<C::Adapter>();
        if self.record.adapter_type != expected {
            return Err(CapabilityContractError::AdapterTypeMismatch {
                capability_id: self.record.id,
                expected,
                actual: self.record.adapter_type,
            });
        }
        let Some(adapter) = self.record.adapter.downcast_ref::<C::Adapter>() else {
            return Err(CapabilityContractError::CorruptAdapter {
                capability_id: self.record.id,
                expected,
            });
        };
        Ok(Capability {
            target: &self.record.target,
            adapter,
            origin: &self.record.origin,
        })
    }
}

/// A capability handle whose adapter type is fixed by its specification.
pub(crate) struct Capability<'ctx, C: CapabilitySpec> {
    target: &'ctx CapabilityTargetId,
    adapter: &'ctx C::Adapter,
    origin: &'ctx CapabilityOrigin,
}

impl<'ctx, C: CapabilitySpec> Capability<'ctx, C> {
    pub(crate) fn target(&self) -> &'ctx CapabilityTargetId {
        self.target
    }

    pub(crate) fn adapter(&self) -> &'ctx C::Adapter {
        self.adapter
    }

    pub(crate) fn origin(&self) -> &'ctx CapabilityOrigin {
        self.origin
    }
}

impl<C: CapabilitySpec> Copy for Capability<'_, C> {}

impl<C: CapabilitySpec> Clone for Capability<'_, C> {
    fn clone(&self) -> Self {
        *self
    }
}
