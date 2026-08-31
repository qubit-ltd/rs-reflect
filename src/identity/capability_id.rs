// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Stable identities for reflection capabilities.

use crate::error::IdError;

/// A stable, namespaced identifier for a reflection capability.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CapabilityId(Box<str>);

impl CapabilityId {
    /// Creates an externally owned capability ID.
    ///
    /// Returns [`IdError`] when `value` is malformed or uses the reserved
    /// `qubit.reflect` namespace.
    pub fn new(value: &str) -> Result<Self, IdError> {
        validate(value, IdAuthority::EXTERNAL)?;
        Ok(Self(value.into()))
    }

    /// Creates a capability ID owned by the reflection library.
    ///
    /// Returns [`IdError`] when `value` is malformed. This crate-private
    /// constructor is reserved for future built-in `qubit.reflect.*`
    /// registrations and must not become a downstream API.
    #[allow(dead_code, reason = "reserved for future built-in capability registrations")]
    pub(crate) fn new_core(value: &str) -> Result<Self, IdError> {
        validate(value, IdAuthority::CORE)?;
        Ok(Self(value.into()))
    }

    /// Returns the stable textual representation of this ID.
    #[must_use]
    #[inline(always)]
    pub fn as_str(&self) -> &str {
        let Self(value) = self;
        value
    }
}

impl AsRef<str> for CapabilityId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl std::fmt::Display for CapabilityId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl std::str::FromStr for CapabilityId {
    type Err = IdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}

/// Determines whether an ID is owned by this crate or an external crate.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct IdAuthority {
    is_core: bool,
}

impl IdAuthority {
    /// Marks an ID as externally owned and ineligible for the reserved
    /// namespace.
    pub(crate) const EXTERNAL: Self = Self { is_core: false };

    /// Marks an ID as owned by this crate and eligible for the reserved
    /// namespace.
    const CORE: Self = Self { is_core: true };
}

/// Validates a namespaced ID according to its owning authority.
pub(crate) fn validate(value: &str, authority: IdAuthority) -> Result<(), IdError> {
    validate_segments(value)?;
    if authority != IdAuthority::CORE && (value == "qubit.reflect" || value.starts_with("qubit.reflect.")) {
        return Err(IdError::ReservedNamespace { value: value.into() });
    }
    Ok(())
}

/// Validates dot-separated ASCII identifier segments.
fn validate_segments(value: &str) -> Result<(), IdError> {
    if value.is_empty()
        || value.split('.').any(|segment| {
            segment.is_empty()
                || !segment.bytes().enumerate().all(|(index, byte)| {
                    matches!(byte, b'a'..=b'z' | b'A'..=b'Z' | b'_') || (index > 0 && byte.is_ascii_digit())
                })
        })
    {
        return Err(IdError::InvalidFormat { value: value.into() });
    }
    Ok(())
}
