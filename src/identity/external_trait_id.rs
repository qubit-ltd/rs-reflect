// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Stable identities for traits that are not reflected themselves.

use std::fmt;
use std::str::FromStr;

use crate::error::IdError;
use crate::identity::capability_id::IdAuthority;
use crate::identity::capability_id::validate;

/// A stable, namespaced identifier for an external trait.
///
/// # Examples
///
/// ```
/// use qubit_reflect::identity::ExternalTraitId;
/// let id = ExternalTraitId::new("example.display").expect("valid trait ID");
/// assert_eq!(id.as_str(), "example.display");
/// ```
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ExternalTraitId(Box<str>);

impl ExternalTraitId {
    /// Creates an externally owned trait ID.
    ///
    /// Returns [`IdError`] when `value` is malformed or uses the reserved
    /// `qubit.reflect` namespace.
    ///
    /// # Parameters
    ///
    /// - `value`: Candidate dot-separated trait identifier.
    ///
    /// # Returns
    ///
    /// Returns the validated external trait ID.
    ///
    /// # Errors
    ///
    /// Returns [`IdError`] when the name is malformed or reserved.
    #[must_use]
    pub fn new(value: &str) -> Result<Self, IdError> {
        validate(value, IdAuthority::EXTERNAL)?;
        Ok(Self(value.into()))
    }

    /// Returns the stable textual representation of this ID.
    ///
    /// # Returns
    ///
    /// Returns the validated identifier string.
    #[must_use]
    #[inline]
    pub fn as_str(&self) -> &str {
        let Self(value) = self;
        value
    }
}

impl AsRef<str> for ExternalTraitId {
    /// Returns the identifier as a string slice.
    ///
    /// # Returns
    ///
    /// Returns the validated identifier text.
    #[must_use]
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for ExternalTraitId {
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

impl FromStr for ExternalTraitId {
    type Err = IdError;

    /// Parses and validates an externally owned trait identifier.
    ///
    /// # Parameters
    ///
    /// - `value`: Candidate dot-separated trait identifier.
    ///
    /// # Returns
    ///
    /// Returns the validated ID.
    ///
    /// # Errors
    ///
    /// Returns [`IdError`] when the identifier is malformed or reserved.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}
