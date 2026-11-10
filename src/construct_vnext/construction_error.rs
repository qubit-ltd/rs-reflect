// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::fmt;

use super::ConstructionAvailability;
use super::ConstructionShape;
use super::ConstructionUnavailableReason;
use crate::runtime::construct as runtime;

/// Public construction error categories and details.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ConstructionError {
    /// The selected target has no registered construction provider.
    MissingProvider,
    /// More than one provider matches the target.
    AmbiguousProvider {
        /// Rust type name of the ambiguous target.
        target: &'static str,
        /// Number of matching providers.
        count: usize,
    },
    /// Construction was requested for a type or descriptor that is not
    /// constructible.
    WrongTargetKind,
    /// The selected target is unavailable for the reported reasons.
    Unavailable(
        /// Structured reasons this target cannot be constructed.
        ConstructionAvailability,
    ),
    /// The requested output type does not match the provider target type.
    TargetTypeMismatch(
        /// Expected target type and type supplied by the caller.
        crate::error::TypeMismatch,
    ),
    /// The input layout does not match the target's construction shape.
    ShapeMismatch {
        /// Shape required by the target.
        expected: ConstructionShape,
        /// Shape supplied by the caller.
        actual: ConstructionShape,
    },
    /// A required field value was omitted.
    MissingField {
        /// Name of the missing field.
        field: &'static str,
    },
    /// A named field was supplied more than once.
    DuplicateField {
        /// Name of the duplicated field.
        field: &'static str,
    },
    /// The input names a field that is not part of the target.
    UnknownField {
        /// Name supplied by the caller.
        field: Box<str>,
    },
    /// The caller supplied a field that only its provider may initialize.
    ProviderOnlyField {
        /// Name of the provider-only field.
        field: &'static str,
    },
    /// The selected field is unavailable for the reported reason.
    FieldUnavailable {
        /// Name of the unavailable field.
        field: &'static str,
        /// Why the field cannot be constructed.
        reason: ConstructionUnavailableReason,
    },
    /// The caller attempted to set a field that is not public.
    FieldNotPublic {
        /// Name of the restricted field.
        field: &'static str,
    },
    /// A supplied field value has the wrong reflected type.
    ValueTypeMismatch {
        /// Name of the field with the wrong value type.
        field: &'static str,
        /// Expected and actual value type identities.
        mismatch: crate::error::TypeMismatch,
    },
    /// Positional input contains more values than the target accepts.
    TooManyValues {
        /// Maximum number of accepted values.
        expected_max: usize,
        /// Number of values supplied.
        actual: usize,
    },
}

impl fmt::Display for ConstructionError {
    /// Formats the structured error using its stable debug representation.
    /// # Parameters
    ///
    /// - `formatter`: Formatter receiving the human-readable representation.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ConstructionError {}

/// Maps every runtime construction error variant to its public counterpart.
/// # Parameters
///
/// - `inner`: Runtime representation being adapted to the public API.
pub(super) fn public_construction_error(inner: &runtime::ConstructionError) -> ConstructionError {
    use runtime::ConstructionError as RuntimeError;
    match inner {
        RuntimeError::MissingProvider => ConstructionError::MissingProvider,
        RuntimeError::AmbiguousProvider { target, count } => ConstructionError::AmbiguousProvider {
            target: target.rust_name,
            count: *count,
        },
        RuntimeError::WrongTargetKind => ConstructionError::WrongTargetKind,
        RuntimeError::Unavailable(reason) => ConstructionError::Unavailable(reason.clone()),
        RuntimeError::TargetTypeMismatch(mismatch) => {
            ConstructionError::TargetTypeMismatch(super::internal::public_mismatch(mismatch))
        }
        RuntimeError::ShapeMismatch { expected, actual } => ConstructionError::ShapeMismatch {
            expected: *expected,
            actual: *actual,
        },
        RuntimeError::MissingField { field } => ConstructionError::MissingField { field },
        RuntimeError::DuplicateField { field } => ConstructionError::DuplicateField { field },
        RuntimeError::UnknownField { field } => ConstructionError::UnknownField { field: field.clone() },
        RuntimeError::ProviderOnlyField { field } => ConstructionError::ProviderOnlyField { field },
        RuntimeError::FieldUnavailable { field, reason } => {
            ConstructionError::FieldUnavailable { field, reason: *reason }
        }
        RuntimeError::FieldNotPublic { field } => ConstructionError::FieldNotPublic { field },
        RuntimeError::ValueTypeMismatch { field, mismatch } => ConstructionError::ValueTypeMismatch {
            field,
            mismatch: super::internal::public_mismatch(mismatch),
        },
        RuntimeError::TooManyValues { expected_max, actual } => ConstructionError::TooManyValues {
            expected_max: *expected_max,
            actual: *actual,
        },
    }
}
