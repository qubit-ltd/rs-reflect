use std::any::TypeId;
use std::fmt;

use crate::error::IdError;
use crate::runtime::error::ContextError;

use super::capability_id::CapabilityIdentifier;
use super::capability_target::CapabilityTargetId;

/// A failure while restoring a concrete capability contract from its ID.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum CapabilityContractError {
    IdMismatch {
        expected: &'static str,
        actual: CapabilityIdentifier,
    },
    AdapterTypeMismatch {
        capability_id: CapabilityIdentifier,
        expected: TypeId,
        actual: TypeId,
    },
    CorruptAdapter {
        capability_id: CapabilityIdentifier,
        expected: TypeId,
    },
}

/// A lookup or validation error for a target-bound capability.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum CapabilityError {
    Context(ContextError),
    Missing {
        target: CapabilityTargetId,
        capability_id: &'static str,
    },
    TargetMismatch {
        expected: CapabilityTargetId,
        actual: CapabilityTargetId,
    },
    ContractMismatch {
        target: CapabilityTargetId,
        capability_id: CapabilityIdentifier,
        expected_adapter: TypeId,
        actual_adapter: TypeId,
        origin: super::capability_origin::CapabilityOrigin,
    },
    Conflict {
        target: CapabilityTargetId,
        capability_id: CapabilityIdentifier,
        registrations: Box<[super::capability_origin::CapabilityOrigin]>,
    },
    Contract(CapabilityContractError),
}

impl fmt::Display for CapabilityContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IdMismatch { expected, actual } => write!(
                formatter,
                "capability ID mismatch: expected {expected}, got {actual}"
            ),
            Self::AdapterTypeMismatch { capability_id, .. } => write!(
                formatter,
                "adapter contract mismatch for capability {capability_id}"
            ),
            Self::CorruptAdapter { capability_id, .. } => write!(
                formatter,
                "invalid stored adapter for capability {capability_id}"
            ),
        }
    }
}

impl std::error::Error for CapabilityContractError {}

impl fmt::Display for CapabilityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Context(error) => write!(formatter, "reflection context failed: {error}"),
            Self::Missing { capability_id, .. } => {
                write!(formatter, "missing capability {capability_id}")
            }
            Self::TargetMismatch { .. } => {
                formatter.write_str("capability registration targets a different reflection entity")
            }
            Self::ContractMismatch { capability_id, .. } => write!(
                formatter,
                "adapter contract mismatch for capability {capability_id}"
            ),
            Self::Conflict { capability_id, .. } => write!(
                formatter,
                "conflicting registrations for capability {capability_id}"
            ),
            Self::Contract(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for CapabilityError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Context(error) => Some(error),
            Self::Contract(error) => Some(error),
            _ => None,
        }
    }
}

/// A capability registration could not be constructed.
#[derive(Debug)]
pub(crate) enum CapabilityRegistrationError {
    InvalidId(IdError),
}

impl fmt::Display for CapabilityRegistrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidId(error) => write!(formatter, "invalid capability ID: {error}"),
        }
    }
}

impl std::error::Error for CapabilityRegistrationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidId(error) => Some(error),
        }
    }
}
