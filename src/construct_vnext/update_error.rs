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
use super::UpdateUnavailableReason;
use crate::runtime::construct as runtime;

/// Public update error categories and details.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum UpdateError {
    /// The supplied base value is not of the type being updated.
    BaseTypeMismatch(
        /// Expected base type and type supplied by the caller.
        crate::error::TypeMismatch,
    ),
    /// The requested output type does not match the provider target type.
    TargetTypeMismatch(
        /// Expected update target type and type requested by the caller.
        crate::error::TypeMismatch,
    ),
    /// Update was requested for a type or descriptor that cannot be updated.
    WrongTargetKind,
    /// The selected target has no registered update provider.
    MissingProvider,
    /// More than one provider matches the target.
    AmbiguousProvider {
        /// Rust type name of the ambiguous target.
        target: &'static str,
        /// Number of matching providers.
        count: usize,
    },
    /// The selected target is unavailable for the reported reasons.
    Unavailable(
        /// Structured reasons this target cannot be updated.
        ConstructionAvailability,
    ),
    /// The input layout does not match the target's update shape.
    ShapeMismatch {
        /// Shape required by the target.
        expected: ConstructionShape,
        /// Shape supplied by the caller.
        actual: ConstructionShape,
    },
    /// A required replacement value was omitted.
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
        /// Why the field cannot be changed.
        reason: ConstructionUnavailableReason,
    },
    /// The caller attempted to replace a field that is not public.
    FieldNotPublic {
        /// Name of the restricted field.
        field: &'static str,
    },
    /// A replacement value has the wrong reflected type.
    ValueTypeMismatch {
        /// Name of the field with the wrong value type.
        field: &'static str,
        /// Expected and actual value type identities.
        mismatch: crate::error::TypeMismatch,
    },
    /// Positional input contains more replacements than the target accepts.
    TooManyValues {
        /// Maximum number of accepted replacements.
        expected_max: usize,
        /// Number of replacements supplied.
        actual: usize,
    },
    /// The caller tried to replace a field whose update policy preserves it.
    FieldPreserved {
        /// Name of the preserved field.
        field: &'static str,
    },
    /// The field cannot be updated for the reported reason.
    FieldUpdateUnavailable {
        /// Name of the unavailable field.
        field: &'static str,
        /// Why the field cannot be changed.
        reason: UpdateUnavailableReason,
    },
    /// The target cannot be updated for the reported reason.
    UpdateUnavailable(
        /// Structured reason the target has no usable update path.
        UpdateUnavailableReason,
    ),
}

impl fmt::Display for UpdateError {
    /// Formats the structured error using its stable debug representation.
    /// # Parameters
    ///
    /// - `formatter`: Formatter receiving the human-readable representation.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for UpdateError {}

/// Maps every runtime update error variant to its public counterpart.
/// # Parameters
///
/// - `inner`: Runtime representation being adapted to the public API.
pub(super) fn public_update_error(inner: &runtime::UpdateError) -> UpdateError {
    use runtime::UpdateError as RuntimeError;
    match inner {
        RuntimeError::BaseTypeMismatch(mismatch) => {
            UpdateError::BaseTypeMismatch(super::internal::public_mismatch(mismatch))
        }
        RuntimeError::TargetTypeMismatch(mismatch) => {
            UpdateError::TargetTypeMismatch(super::internal::public_mismatch(mismatch))
        }
        RuntimeError::WrongTargetKind => UpdateError::WrongTargetKind,
        RuntimeError::MissingProvider => UpdateError::MissingProvider,
        RuntimeError::AmbiguousProvider { target, count } => UpdateError::AmbiguousProvider {
            target: target.rust_name,
            count: *count,
        },
        RuntimeError::Unavailable(reason) => UpdateError::Unavailable(reason.clone()),
        RuntimeError::ShapeMismatch { expected, actual } => UpdateError::ShapeMismatch {
            expected: *expected,
            actual: *actual,
        },
        RuntimeError::MissingField { field } => UpdateError::MissingField { field },
        RuntimeError::DuplicateField { field } => UpdateError::DuplicateField { field },
        RuntimeError::UnknownField { field } => UpdateError::UnknownField { field: field.clone() },
        RuntimeError::ProviderOnlyField { field } => UpdateError::ProviderOnlyField { field },
        RuntimeError::FieldUnavailable { field, reason } => UpdateError::FieldUnavailable { field, reason: *reason },
        RuntimeError::FieldNotPublic { field } => UpdateError::FieldNotPublic { field },
        RuntimeError::ValueTypeMismatch { field, mismatch } => UpdateError::ValueTypeMismatch {
            field,
            mismatch: super::internal::public_mismatch(mismatch),
        },
        RuntimeError::TooManyValues { expected_max, actual } => UpdateError::TooManyValues {
            expected_max: *expected_max,
            actual: *actual,
        },
        RuntimeError::FieldPreserved { field } => UpdateError::FieldPreserved { field },
        RuntimeError::FieldUpdateUnavailable { field, reason } => {
            UpdateError::FieldUpdateUnavailable { field, reason: *reason }
        }
        RuntimeError::UpdateUnavailable(reason) => UpdateError::UpdateUnavailable(*reason),
    }
}
