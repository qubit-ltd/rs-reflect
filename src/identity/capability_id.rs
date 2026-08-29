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

    /// Creates a capability ID for use by the reflection library itself.
    ///
    /// Returns [`IdError`] when `value` is not a dot-separated ASCII identifier.
    #[doc(hidden)]
    pub fn new_core(value: &str) -> Result<Self, IdError> {
        validate(value, IdAuthority::CORE)?;
        Ok(Self(value.into()))
    }

    /// Returns the stable textual representation of this ID.
    pub fn as_str(&self) -> &str {
        &self.0
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

/// Determines whether an ID is owned by the reflection library or an external crate.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct IdAuthority {
    is_core: bool,
}

impl IdAuthority {
    /// Marks an ID as externally owned and therefore ineligible for the reserved namespace.
    pub const EXTERNAL: Self = Self { is_core: false };

    /// Marks an ID as owned by this crate and eligible for the reserved namespace.
    pub(crate) const CORE: Self = Self { is_core: true };
}

/// Validates an ID according to its owner.
pub(crate) fn validate(value: &str, authority: IdAuthority) -> Result<(), IdError> {
    validate_segments(value)?;
    if authority != IdAuthority::CORE
        && (value == "qubit.reflect" || value.starts_with("qubit.reflect."))
    {
        return Err(IdError::ReservedNamespace {
            value: value.into(),
        });
    }
    Ok(())
}

/// Validates dot-separated ASCII identifier segments.
fn validate_segments(value: &str) -> Result<(), IdError> {
    if value.is_empty()
        || value.split('.').any(|segment| {
            segment.is_empty()
                || !segment.bytes().enumerate().all(|(index, byte)| {
                    matches!(byte, b'a'..=b'z' | b'A'..=b'Z' | b'_')
                        || (index > 0 && byte.is_ascii_digit())
                })
        })
    {
        return Err(IdError::InvalidFormat {
            value: value.into(),
        });
    }
    Ok(())
}
