// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

pub(crate) use crate::identity::CapabilityId as CapabilityIdentifier;

pub(crate) fn validate_capability_id(value: &'static str) -> Result<CapabilityIdentifier, crate::error::IdError> {
    CapabilityIdentifier::new(value)
}
