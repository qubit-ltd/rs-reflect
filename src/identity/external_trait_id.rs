// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Stable identities for traits that are not reflected themselves.

use crate::error::IdError;
use crate::identity::capability_id::IdAuthority;
use crate::identity::capability_id::validate;

/// A stable, namespaced identifier for an external trait.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ExternalTraitId(Box<str>);

impl ExternalTraitId {
    /// Creates an externally owned trait ID.
    ///
    /// Returns [`IdError`] when `value` is malformed or uses the reserved
    /// `qubit.reflect` namespace.
    pub fn new(value: &str) -> Result<Self, IdError> {
        validate(value, IdAuthority::EXTERNAL)?;
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

impl AsRef<str> for ExternalTraitId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl std::fmt::Display for ExternalTraitId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl std::str::FromStr for ExternalTraitId {
    type Err = IdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}
