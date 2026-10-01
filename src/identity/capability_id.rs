// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Stable identities for reflection capabilities.

use std::fmt;

use crate::error::IdError;

/// A stable, namespaced identifier for a reflection capability.
///
/// # Examples
///
/// ```
/// use qubit_reflect::identity::CapabilityId;
/// let id = CapabilityId::new("example.cache").expect("valid capability ID");
/// assert_eq!(id.as_str(), "example.cache");
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CapabilityId(&'static str);

impl CapabilityId {
    /// Validates a potentially dynamic external capability name without
    /// promoting it to a static ABI identity.
    ///
    /// # Parameters
    ///
    /// - `value`: Candidate dot-separated capability name.
    ///
    /// # Returns
    ///
    /// Returns `()` when `value` is a valid external capability ID.
    ///
    /// # Errors
    ///
    /// Returns [`IdError`] when the name is malformed or uses the reserved
    /// `qubit.reflect` namespace.
    pub fn validate(value: &str) -> Result<(), IdError> {
        validate(value, IdAuthority::EXTERNAL)
    }

    /// Creates an externally defined static capability ID.
    ///
    /// Returns [`IdError`] when `value` is malformed or uses the reserved
    /// `qubit.reflect` namespace.
    ///
    /// # Parameters
    ///
    /// - `value`: Static candidate capability name.
    ///
    /// # Returns
    ///
    /// Returns the static external capability ID.
    ///
    /// # Errors
    ///
    /// Returns [`IdError`] when the name is malformed or reserved.
    pub fn new(value: &'static str) -> Result<Self, IdError> {
        Self::validate(value)?;
        Ok(Self(value))
    }

    /// Creates a capability ID owned by the reflection library.
    ///
    /// Returns [`IdError`] when `value` is malformed. This crate-private
    /// constructor is reserved for future built-in `qubit.reflect.*`
    /// registrations and must not become a downstream API.
    ///
    /// # Parameters
    ///
    /// - `value`: Static candidate core capability name.
    ///
    /// # Returns
    ///
    /// Returns the static core capability ID.
    ///
    /// # Errors
    ///
    /// Returns [`IdError`] when the name is malformed.
    #[allow(dead_code, reason = "reserved for future built-in capability registrations")]
    pub(crate) fn new_core(value: &'static str) -> Result<Self, IdError> {
        validate(value, IdAuthority::CORE)?;
        Ok(Self(value))
    }

    /// Returns the stable textual representation of this ID.
    ///
    /// # Returns
    ///
    /// Returns the validated identifier text.
    #[must_use]
    #[inline]
    pub fn as_str(&self) -> &str {
        let Self(value) = self;
        value
    }
}

impl AsRef<str> for CapabilityId {
    /// Returns the capability identifier as a string slice.
    ///
    /// # Returns
    ///
    /// Returns the stable identifier text.
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for CapabilityId {
    /// Formats the stable identifier text.
    ///
    /// # Parameters
    ///
    /// - `formatter`: Formatter receiving the identifier.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after writing the identifier, or the formatter error.
    ///
    /// # Errors
    ///
    /// Returns an error reported by the formatter.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Determines whether an ID is owned by this crate or an external crate.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct IdAuthority {
    /// Whether IDs using the reserved core namespace are accepted.
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
///
/// # Parameters
///
/// - `value`: Candidate dot-separated identifier.
/// - `authority`: Whether the identifier belongs to the core or an external
///   crate.
///
/// # Returns
///
/// Returns `()` when every segment is valid and the namespace is permitted.
///
/// # Errors
///
/// Returns [`IdError`] when the identifier is malformed or uses a reserved
/// namespace.
pub(crate) fn validate(value: &str, authority: IdAuthority) -> Result<(), IdError> {
    validate_segments(value)?;
    if authority != IdAuthority::CORE && (value == "qubit.reflect" || value.starts_with("qubit.reflect.")) {
        return Err(IdError::ReservedNamespace { value: value.into() });
    }
    Ok(())
}

/// Validates dot-separated ASCII identifier segments.
///
/// # Parameters
///
/// - `value`: Candidate identifier whose dot-separated segments are checked.
///
/// # Returns
///
/// Returns `()` when each segment is a valid ASCII identifier.
///
/// # Errors
///
/// Returns [`IdError::InvalidFormat`] for an empty name or invalid segment.
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
