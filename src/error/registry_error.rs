// qubit-style: allow public-type-layout
//! Immutable errors reported while aggregating registration fragments.

use std::sync::Arc;

use crate::identity::FragmentIdentity;

/// The machine-readable class of a registry aggregation error.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RegistryErrorKind {
    /// Two registration fragments claim the same identity.
    DuplicateFragment,
    /// Fragment facts disagree about an identity's content.
    IdentityConflict,
    /// External trait registrations use an incompatible ID.
    ExternalTraitIdConflict,
    /// Capability registrations use an incompatible ID or contract.
    CapabilityConflict,
    /// The target platform cannot support distributed registration.
    UnsupportedPlatform,
}

/// A shareable immutable registry aggregation error.
#[derive(Clone, Debug)]
pub struct RegistryError(Arc<RegistryErrorData>);

#[derive(Debug)]
struct RegistryErrorData {
    kind: RegistryErrorKind,
    left: Option<FragmentIdentity>,
    right: Option<FragmentIdentity>,
}

impl RegistryError {
    /// Creates an error for two fragments that claim the same registration
    /// identity.
    pub fn duplicate_fragment(left: FragmentIdentity, right: FragmentIdentity) -> Self {
        Self::conflict(RegistryErrorKind::DuplicateFragment, left, right)
    }

    /// Creates an error for fragments that disagree about one identity's
    /// content.
    pub fn identity_conflict(left: FragmentIdentity, right: FragmentIdentity) -> Self {
        Self::conflict(RegistryErrorKind::IdentityConflict, left, right)
    }

    /// Creates an error for incompatible external-trait registrations.
    pub fn external_trait_id_conflict(left: FragmentIdentity, right: FragmentIdentity) -> Self {
        Self::conflict(RegistryErrorKind::ExternalTraitIdConflict, left, right)
    }

    /// Creates an error for incompatible capability registrations.
    pub fn capability_conflict(left: FragmentIdentity, right: FragmentIdentity) -> Self {
        Self::conflict(RegistryErrorKind::CapabilityConflict, left, right)
    }

    /// Creates an error when the current platform lacks
    /// distributed-registration support.
    pub fn unsupported_platform() -> Self {
        Self(Arc::new(RegistryErrorData {
            kind: RegistryErrorKind::UnsupportedPlatform,
            left: None,
            right: None,
        }))
    }

    /// Returns the stable machine-readable error category.
    pub fn kind(&self) -> RegistryErrorKind {
        self.0.kind
    }

    /// Returns the two conflicting fragments when this error originated from a
    /// conflict.
    pub fn conflicting_fragments(&self) -> Option<(&FragmentIdentity, &FragmentIdentity)> {
        match (&self.0.left, &self.0.right) {
            (Some(left), Some(right)) => Some((left, right)),
            _ => None,
        }
    }

    /// Creates a conflict error retaining both conflicting registration
    /// fragments.
    fn conflict(kind: RegistryErrorKind, left: FragmentIdentity, right: FragmentIdentity) -> Self {
        Self(Arc::new(RegistryErrorData {
            kind,
            left: Some(left),
            right: Some(right),
        }))
    }
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "reflection registry error: {:?}", self.kind())
    }
}

impl std::error::Error for RegistryError {}
