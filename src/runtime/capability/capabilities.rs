use std::any::TypeId;
use std::collections::{BTreeMap, HashMap};

use super::capability::{CapabilityDescriptor, CapabilitySpec};
use super::capability_error::CapabilityError;
use super::capability_id::CapabilityIdentifier;
use super::capability_origin::CapabilityOrigin;
use super::capability_registration::CapabilityRegistration;
use super::capability_target::CapabilityTargetId;

pub(crate) struct CapabilityRecord {
    pub(super) target: CapabilityTargetId,
    pub(super) id: CapabilityIdentifier,
    pub(super) adapter_type: TypeId,
    pub(super) adapter: std::sync::Arc<dyn std::any::Any + Send + Sync>,
    pub(super) origin: CapabilityOrigin,
}

/// An immutable capability set belonging to one reflection target.
pub struct Capabilities {
    pub(super) target: CapabilityTargetId,
    pub(super) records: Box<[CapabilityRecord]>,
}

impl Capabilities {
    pub(crate) fn empty(target: CapabilityTargetId) -> Self {
        Self {
            target,
            records: Box::new([]),
        }
    }

    pub(crate) fn try_new(
        target: CapabilityTargetId,
        registrations: impl IntoIterator<Item = CapabilityRegistration>,
    ) -> Result<Self, CapabilityError> {
        let mut by_id = HashMap::<CapabilityIdentifier, CapabilityRecord>::new();
        let mut conflicts = BTreeMap::<CapabilityIdentifier, Vec<CapabilityOrigin>>::new();
        for registration in registrations {
            if registration.target != target {
                return Err(CapabilityError::TargetMismatch {
                    expected: target,
                    actual: registration.target,
                });
            }
            let adapter_type = registration.adapter.as_ref().type_id();
            let record = CapabilityRecord {
                target: registration.target,
                id: registration.id,
                adapter_type,
                adapter: std::sync::Arc::from(registration.adapter),
                origin: registration.origin,
            };
            if let Some(previous) = by_id.remove(&record.id) {
                let origins = conflicts
                    .entry(record.id)
                    .or_insert_with(|| vec![previous.origin]);
                origins.push(record.origin);
            } else if let Some(origins) = conflicts.get_mut(&record.id) {
                origins.push(record.origin);
            } else {
                by_id.insert(record.id, record);
            }
        }
        if let Some((capability_id, registrations)) = conflicts.into_iter().next() {
            return Err(CapabilityError::Conflict {
                target,
                capability_id,
                registrations: registrations.into_boxed_slice(),
            });
        }
        let mut records: Vec<_> = by_id.into_values().collect();
        records.sort_by(|left, right| left.id.as_str().cmp(right.id.as_str()));
        Ok(Self {
            target,
            records: records.into_boxed_slice(),
        })
    }

    pub(crate) fn target(&self) -> &CapabilityTargetId {
        &self.target
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn get(&self, id: &str) -> Option<CapabilityDescriptor<'_>> {
        self.records
            .iter()
            .find(|record| record.id.as_str() == id)
            .map(|record| CapabilityDescriptor { record })
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = CapabilityDescriptor<'_>> + '_ {
        self.records
            .iter()
            .map(|record| CapabilityDescriptor { record })
    }

    pub(crate) fn find<'ctx, C: CapabilitySpec>(
        &self,
    ) -> Result<Option<super::capability::Capability<'ctx, C>>, CapabilityError> {
        let Some(descriptor) = self.get(C::ID) else {
            return Ok(None);
        };
        descriptor
            .downcast::<C>()
            .map(Some)
            .map_err(|error| match error {
                super::capability_error::CapabilityContractError::AdapterTypeMismatch {
                    capability_id,
                    expected,
                    actual,
                } => CapabilityError::ContractMismatch {
                    target: self.target.clone(),
                    capability_id,
                    expected_adapter: expected,
                    actual_adapter: actual,
                    origin: descriptor.origin().clone(),
                },
                other => CapabilityError::Contract(other),
            })
    }

    pub(crate) fn capability<'ctx, C: CapabilitySpec>(
        &self,
    ) -> Result<super::capability::Capability<'ctx, C>, CapabilityError> {
        self.find::<C>()?.ok_or_else(|| CapabilityError::Missing {
            target: self.target.clone(),
            capability_id: C::ID,
        })
    }
}
