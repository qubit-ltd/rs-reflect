use std::any::TypeId;

use super::capabilities::CapabilityRecord;
use super::capability_error::CapabilityContractError;
use super::capability_origin::CapabilityOrigin;
use super::capability_target::CapabilityTargetId;

/// Binds a stable capability ID to exactly one Rust adapter contract.
pub trait CapabilitySpec: 'static {
    type Adapter: Send + Sync + 'static;

    const ID: &'static str;
}

/// A context-independent descriptor for finding a typed adapter.
#[derive(Clone, Copy)]
pub struct CapabilityDescriptor<'ctx> {
    pub(super) record: &'ctx CapabilityRecord,
}

impl<'ctx> CapabilityDescriptor<'ctx> {
    pub fn id(self) -> &'ctx str {
        self.record.id.as_str()
    }

    pub fn adapter_type_id(self) -> TypeId {
        self.record.adapter_type
    }

    pub fn target(self) -> &'ctx CapabilityTargetId {
        &self.record.target
    }

    pub fn origin(self) -> &'ctx CapabilityOrigin {
        &self.record.origin
    }

    pub fn downcast<'cap, C: CapabilitySpec>(
        self,
    ) -> Result<Capability<'cap, C>, CapabilityContractError> {
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
        let adapter = match self.record.adapter.clone().downcast::<C::Adapter>() {
            Ok(adapter) => adapter,
            Err(_) => {
            return Err(CapabilityContractError::CorruptAdapter {
                capability_id: self.record.id,
                expected,
            });
            }
        };
        Ok(Capability {
            target: self.record.target.clone(),
            adapter,
            origin: self.record.origin.clone(),
            _context: std::marker::PhantomData,
        })
    }
}

/// A capability handle whose adapter type is fixed by its specification.
pub struct Capability<'ctx, C: CapabilitySpec> {
    target: CapabilityTargetId,
    adapter: std::sync::Arc<C::Adapter>,
    origin: CapabilityOrigin,
    _context: std::marker::PhantomData<&'ctx ()>,
}

impl<'ctx, C: CapabilitySpec> Capability<'ctx, C> {
    pub fn target(&self) -> &CapabilityTargetId {
        &self.target
    }

    pub fn adapter(&self) -> &C::Adapter {
        &self.adapter
    }

    pub fn origin(&self) -> &CapabilityOrigin {
        &self.origin
    }
}

impl<C: CapabilitySpec> Clone for Capability<'_, C> {
    fn clone(&self) -> Self {
        Self {
            target: self.target.clone(),
            adapter: self.adapter.clone(),
            origin: self.origin.clone(),
            _context: std::marker::PhantomData,
        }
    }
}
