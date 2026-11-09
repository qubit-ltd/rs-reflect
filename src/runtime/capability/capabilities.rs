// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::Arc;

use super::capability_error::CapabilityError;
use super::capability_id::CapabilityIdentifier;
use super::capability_record::CapabilityRecord;
use super::capability_registration_origin::CapabilityRegistrationOrigin;
use super::capability_target_id::CapabilityTargetId;
use crate::runtime::identity::RegistrationOrigin;

/// Immutable, context-independent storage shared by reflection contexts.
pub(crate) struct Capabilities {
    /// Stable identity of the target that owns this capability set.
    pub(super) target: CapabilityTargetId,
    /// Registrations stored in stable capability ID order.
    pub(super) records: Box<[CapabilityRecord]>,
}

impl Capabilities {
    /// Creates an empty set for the specified target.
    ///
    /// # Parameters
    /// - `target`: target identity associated with the set.
    pub(crate) fn empty(target: CapabilityTargetId) -> Self {
        Self {
            target,
            records: Box::new([]),
        }
    }

    /// Creates a validated set and rejects duplicate capability IDs.
    ///
    /// # Parameters
    /// - `target`: target identity that all registrations must reference.
    /// - `registrations`: capability registrations to validate and store.
    pub(crate) fn try_new(
        target: CapabilityTargetId,
        registrations: impl IntoIterator<Item = super::capability_registration::CapabilityRegistration>,
    ) -> Result<Self, CapabilityError> {
        let mut by_id = HashMap::<CapabilityIdentifier, CapabilityRecord>::new();
        let mut conflicts = BTreeMap::<CapabilityIdentifier, Vec<CapabilityRegistrationOrigin>>::new();
        for registration in registrations {
            if registration.target != target {
                return Err(CapabilityError::TargetMismatch {
                    expected: target,
                    actual: registration.target,
                });
            }
            let adapter_type = registration.adapter.as_ref().type_id();
            let record = CapabilityRecord {
                id: registration.id,
                id_str: registration.id_str,
                adapter_type,
                adapter: Arc::from(registration.adapter),
                origin: registration.origin,
            };
            if let Some(previous) = by_id.remove(&record.id) {
                let origins = conflicts.entry(record.id).or_insert_with(|| vec![previous.origin]);
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
}

/// Creates provenance metadata for a built-in capability registration.
pub(crate) fn synthetic_registration_origin() -> RegistrationOrigin {
    RegistrationOrigin {
        crate_name: "qubit-reflect",
        crate_version: None,
        module_path: "intrinsic capability",
        file: None,
        line: None,
        column: None,
    }
}
