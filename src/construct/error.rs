// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Structured construction validation errors.

use std::any::TypeId;
use std::fmt;

use crate::descriptor::FieldDescriptor;
use crate::error::TypeMismatch;

/// The input shape selected for one reflected construction attempt.
///
/// # Examples
///
/// ```
/// use qubit_reflect::construct::ConstructionShape;
///
/// assert_eq!(ConstructionShape::Named.to_string(), "named");
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ConstructionShape {
    /// Query-name bindings for named fields.
    Named,
    /// Source-ordered positional values.
    Tuple,
    /// No field values.
    Unit,
}

impl fmt::Display for ConstructionShape {
    /// Formats the stable shape name used in diagnostics.
    ///
    /// # Parameters
    ///
    /// - `formatter`: The destination formatter.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after writing the shape name.
    ///
    /// # Errors
    ///
    /// Returns a formatting error if writing to `formatter` fails.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Named => formatter.write_str("named"),
            Self::Tuple => formatter.write_str("tuple"),
            Self::Unit => formatter.write_str("unit"),
        }
    }
}

/// Why a generated construction path is not available.
///
/// # Examples
///
/// ```
/// use qubit_reflect::construct::ConstructionUnavailableReason;
///
/// assert_eq!(ConstructionUnavailableReason::UpdateForbidden.to_string(), "field update is unavailable");
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ConstructionUnavailableReason {
    /// A skipped or no-construct field has no explicit value provider.
    MissingDefaultProvider,
    /// A declaration-level symbolic field has no exact runtime identity.
    SymbolicFieldType,
    /// The field source requires its generated provider, not caller input.
    CallerValueForbidden,
    /// The generated update adapter does not permit replacing this field.
    UpdateForbidden,
}

impl fmt::Display for ConstructionUnavailableReason {
    /// Formats a concise human-readable reason.
    ///
    /// # Parameters
    ///
    /// - `formatter`: The destination formatter.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after writing the reason.
    ///
    /// # Errors
    ///
    /// Returns a formatting error if writing to `formatter` fails.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingDefaultProvider => formatter.write_str("missing explicit default provider"),
            Self::SymbolicFieldType => formatter.write_str("field type is not concrete"),
            Self::CallerValueForbidden => formatter.write_str("field value must come from its generated provider"),
            Self::UpdateForbidden => formatter.write_str("field update is unavailable"),
        }
    }
}

/// Stable identity for a field participating in reflected construction.
///
/// # Examples
///
/// ```
/// use qubit_reflect::construct::ConstructionError;
/// use qubit_reflect::construct::NamedConstructionInput;
/// use qubit_reflect::value::DynamicOwned;
/// use qubit_reflect::value::Local;
/// use qubit_reflect::Reflect;
/// use qubit_reflect::TypeDescriptor;
///
/// #[derive(Reflect)]
/// struct User {
///     name: String,
/// }
///
/// let constructor = TypeDescriptor::of::<User>()
///     .struct_construction()
///     .expect("derived construction")
///     .local_constructor();
/// let input = NamedConstructionInput::<Local>::new(std::iter::empty::<(&str, DynamicOwned<Local>)>());
/// let recovery = match constructor.construct_named(input) {
///     Err(recovery) => recovery,
///     Ok(_) => panic!("expected missing-field error"),
/// };
/// let ConstructionError::MissingField { field } = recovery.error() else {
///     panic!("expected missing-field error");
/// };
/// assert_eq!(field.to_string(), "User::name");
/// ```
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ConstructionFieldId {
    /// Exact runtime identity of the type that declared the field.
    declaring_type: TypeId,
    /// Static type name retained for diagnostics.
    declaring_type_name: &'static str,
    /// Zero-based source position among the declaring type's fields.
    index: usize,
    /// Source Rust identifier, absent for positional fields.
    rust_name: Option<&'static str>,
    /// Reflected query identifier, absent for positional fields.
    query_name: Option<&'static str>,
    /// Containing enum variant index, absent for struct fields.
    variant_index: Option<usize>,
    /// Containing enum variant's source identifier, absent for struct fields.
    variant_rust_name: Option<&'static str>,
}

impl ConstructionFieldId {
    /// Captures the public identity facts from an immutable field descriptor.
    ///
    /// # Parameters
    ///
    /// - `field`: The immutable descriptor whose source identity is captured.
    ///
    /// # Returns
    ///
    /// Returns a stable copy of the field's construction identity.
    pub(crate) fn from_descriptor(field: &FieldDescriptor) -> Self {
        let declaring_type = field.declaring_type();
        Self {
            declaring_type: declaring_type.type_id(),
            declaring_type_name: declaring_type.type_name(),
            index: field.index(),
            rust_name: field.rust_name(),
            query_name: field.query_name(),
            variant_index: field.variant_index(),
            variant_rust_name: field.variant_rust_name(),
        }
    }

    /// Returns the declaring root's exact process-local Rust identity.
    ///
    /// # Returns
    ///
    /// Returns the declaring type's process-local `TypeId`.
    #[must_use]
    #[inline]
    pub const fn declaring_type(&self) -> TypeId {
        self.declaring_type
    }

    /// Returns the declaring root's diagnostic Rust type name.
    ///
    /// # Returns
    ///
    /// Returns the static diagnostic type name.
    #[must_use]
    #[inline]
    pub const fn declaring_type_name(&self) -> &'static str {
        self.declaring_type_name
    }

    /// Returns the zero-based source field index.
    ///
    /// # Returns
    ///
    /// Returns the field's source-order index.
    #[must_use]
    #[inline]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Returns the source Rust field name, or `None` for positional fields.
    ///
    /// # Returns
    ///
    /// Returns the source identifier, or `None` when the field is positional.
    #[must_use]
    #[inline]
    pub const fn rust_name(&self) -> Option<&'static str> {
        self.rust_name
    }

    /// Returns the reflected query name, or `None` for positional fields.
    ///
    /// # Returns
    ///
    /// Returns the query name, or `None` when the field is positional.
    #[must_use]
    #[inline]
    pub const fn query_name(&self) -> Option<&'static str> {
        self.query_name
    }

    /// Returns the containing variant index for an enum field.
    ///
    /// # Returns
    ///
    /// Returns the enum variant index, or `None` for a struct field.
    #[must_use]
    #[inline]
    pub const fn variant_index(&self) -> Option<usize> {
        self.variant_index
    }

    /// Returns the containing variant Rust name for an enum field.
    ///
    /// # Returns
    ///
    /// Returns the variant source identifier, or `None` for a struct field.
    #[must_use]
    #[inline]
    pub const fn variant_rust_name(&self) -> Option<&'static str> {
        self.variant_rust_name
    }
}

impl fmt::Display for ConstructionFieldId {
    /// Formats source identity while retaining renamed query names separately.
    ///
    /// # Parameters
    ///
    /// - `formatter`: The destination formatter.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after formatting the source identity.
    ///
    /// # Errors
    ///
    /// Returns a formatting error if writing to `formatter` fails.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.variant_rust_name, self.rust_name) {
            (Some(variant), Some(field)) => {
                write!(formatter, "{}::{variant}.{field}", self.declaring_type_name)
            }
            (Some(variant), None) => write!(
                formatter,
                "{}::{variant} field #{}",
                self.declaring_type_name, self.index
            ),
            (None, Some(field)) => {
                write!(formatter, "{}::{field}", self.declaring_type_name)
            }
            (None, None) => write!(formatter, "{} field #{}", self.declaring_type_name, self.index),
        }
    }
}

/// A machine-readable reason reflected construction failed before execution.
///
/// # Examples
///
/// ```
/// use qubit_reflect::construct::{ConstructionError, ConstructionShape};
///
/// let error = ConstructionError::WrongShape {
///     expected: ConstructionShape::Named,
///     actual: ConstructionShape::Tuple,
/// };
/// assert!(error.to_string().contains("shape mismatch"));
/// ```
#[must_use]
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ConstructionError {
    /// The reflected target did not generate the requested construction path.
    #[error("dynamic construction is unavailable for this target")]
    TargetUnavailable,
    /// The selected input form does not match the struct or variant shape.
    #[error("construction shape mismatch: expected {expected}, got {actual}")]
    WrongShape {
        /// Shape declared by the target descriptor.
        expected: ConstructionShape,
        /// Shape selected by the caller.
        actual: ConstructionShape,
    },
    /// A named binding does not match a declared query name.
    #[error("unknown construction field `{name}`")]
    UnknownField {
        /// Unrecognized query name retained verbatim.
        name: Box<str>,
    },
    /// One named binding appeared more than once.
    #[error("duplicate construction field `{name}`")]
    DuplicateField {
        /// Repeated query name retained verbatim.
        name: Box<str>,
    },
    /// A required field has no supplied value or explicit provider.
    #[error("missing required construction field {field}")]
    MissingField {
        /// Exact declared field identity.
        field: Box<ConstructionFieldId>,
    },
    /// Positional input contains a value beyond the declared field range.
    #[error("unknown construction field position {index}")]
    UnknownPosition {
        /// First zero-based position outside the field range.
        index: usize,
    },
    /// A field value has the wrong exact Rust type.
    #[error("construction value type mismatch for {field}: {mismatch}")]
    ValueTypeMismatch {
        /// Exact declared field identity.
        field: Box<ConstructionFieldId>,
        /// Expected and actual process-local type identities.
        mismatch: TypeMismatch,
    },
    /// An update base has the wrong exact reflected root type.
    #[error("construction update base type mismatch: {mismatch}")]
    BaseTypeMismatch {
        /// Expected and actual process-local type identities.
        mismatch: TypeMismatch,
    },
    /// Descriptor policy prevents the target construction path from running.
    #[error("construction unavailable at {field}: {reason}")]
    Unavailable {
        /// Field responsible for the unavailable path.
        field: Box<ConstructionFieldId>,
        /// Stable policy reason.
        reason: ConstructionUnavailableReason,
    },
}
