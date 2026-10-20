// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::TypeId;
use std::fmt;

use super::capability_target_id::CapabilityTargetId;
use crate::error::IdError;
use crate::runtime::error::ContextError;

/// A failure while restoring a concrete capability contract from its ID.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CapabilityContractError {
    /// The requested capability ID differs from the stored registration ID.
    IdMismatch {
        /// The ID required by the requested capability specification.
        expected: &'static str,
        /// The ID recorded in the descriptor.
        actual: crate::identity::CapabilityId,
    },
    /// The adapter's concrete Rust type differs from the capability contract.
    AdapterTypeMismatch {
        /// ID of the capability whose adapter failed validation.
        capability_id: crate::identity::CapabilityId,
        /// Adapter type required by the requested specification.
        expected: TypeId,
        /// Adapter type recorded by the registration.
        actual: TypeId,
    },
    /// A stored adapter could not be downcast despite matching its recorded
    /// type.
    CorruptAdapter {
        /// ID of the capability whose stored adapter is invalid.
        capability_id: crate::identity::CapabilityId,
        /// Adapter type expected by the requested specification.
        expected: TypeId,
    },
}

/// The execution mode used when invoking a built-in capability adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapabilityInvocationMode {
    /// Invocation uses the local, non-thread-safe dynamic value mode.
    Local,
    /// Invocation uses the thread-safe dynamic value mode.
    ThreadSafe,
}

/// A built-in adapter could not operate on the supplied dynamic value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CapabilityInvocationError {
    /// The supplied dynamic value has a different concrete Rust type.
    TypeMismatch {
        /// Concrete value type expected by the adapter.
        expected: TypeId,
        /// Concrete value type supplied to the adapter.
        actual: TypeId,
    },
    /// The adapter has no implementation for the requested value mode.
    UnsupportedMode {
        /// Concrete type handled by the adapter.
        expected_type: TypeId,
        /// Value mode requested by the operation.
        mode: CapabilityInvocationMode,
    },
}

impl fmt::Display for CapabilityInvocationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TypeMismatch { expected, actual } => write!(
                formatter,
                "capability adapter expected value type {expected:?}, got {actual:?}"
            ),
            Self::UnsupportedMode { expected_type, mode } => write!(
                formatter,
                "capability adapter for {expected_type:?} does not support {mode:?} values"
            ),
        }
    }
}

impl std::error::Error for CapabilityInvocationError {}

/// A lookup or validation error for a target-bound capability.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CapabilityError {
    /// The reflection context could not provide the requested metadata.
    Context(ContextError),
    /// No registration exists for this capability ID and target.
    Missing {
        /// Target on which the capability was requested.
        target: CapabilityTargetId,
        /// Stable textual identifier that was not found.
        capability_id: &'static str,
    },
    /// The registration is bound to a different target.
    TargetMismatch {
        /// Target expected by the lookup.
        expected: CapabilityTargetId,
        /// Target bound to the found registration.
        actual: CapabilityTargetId,
    },
    /// A registration's adapter type does not satisfy the capability contract.
    ContractMismatch {
        /// Target whose registration failed validation.
        target: CapabilityTargetId,
        /// ID of the capability whose adapter failed validation.
        capability_id: crate::identity::CapabilityId,
        /// Adapter type required by the requested specification.
        expected_adapter: TypeId,
        /// Adapter type recorded by the registration.
        actual_adapter: TypeId,
        /// Registration source associated with the invalid adapter.
        origin: super::capability_registration_origin::CapabilityRegistrationOrigin,
    },
    /// Multiple registrations conflict for the same target and capability.
    Conflict {
        /// Target for which conflicting registrations were found.
        target: CapabilityTargetId,
        /// ID shared by the conflicting registrations.
        capability_id: crate::identity::CapabilityId,
        /// Sources of all conflicting registrations.
        registrations: Box<[super::capability_registration_origin::CapabilityRegistrationOrigin]>,
    },
    /// The capability descriptor did not satisfy the requested typed contract.
    Contract(CapabilityContractError),
}

impl fmt::Display for CapabilityContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IdMismatch { expected, actual } => {
                write!(formatter, "capability ID mismatch: expected {expected}, got {actual}")
            }
            Self::AdapterTypeMismatch { capability_id, .. } => {
                write!(formatter, "adapter contract mismatch for capability {capability_id}")
            }
            Self::CorruptAdapter { capability_id, .. } => {
                write!(formatter, "invalid stored adapter for capability {capability_id}")
            }
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
            Self::ContractMismatch { capability_id, .. } => {
                write!(formatter, "adapter contract mismatch for capability {capability_id}")
            }
            Self::Conflict { capability_id, .. } => {
                write!(formatter, "conflicting registrations for capability {capability_id}")
            }
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
