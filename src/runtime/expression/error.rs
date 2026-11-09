// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::fmt;

/// The semantic kind of a concrete generic argument.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GenericArgumentKind {
    /// A type parameter argument.
    Type,
    /// A const parameter argument.
    Const,
}

/// A request to view a generic argument through the wrong kind-specific API.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GenericArgumentError {
    /// The kind-specific operation that the caller attempted.
    expected: GenericArgumentKind,
    /// The kind of the argument that was actually supplied.
    actual: GenericArgumentKind,
}

impl GenericArgumentError {
    /// Constructs the mismatch returned by a kind-specific generic argument
    /// accessor.
    ///
    /// # Parameters
    ///
    /// * `expected` — the kind required by the attempted accessor.
    /// * `actual` — the kind represented by the argument.
    pub(crate) const fn new(expected: GenericArgumentKind, actual: GenericArgumentKind) -> Self {
        Self { expected, actual }
    }

    /// Returns the argument kind required by the attempted operation.
    pub const fn expected(self) -> GenericArgumentKind {
        self.expected
    }

    /// Returns the actual kind of the supplied generic argument.
    pub const fn actual(self) -> GenericArgumentKind {
        self.actual
    }
}

impl fmt::Display for GenericArgumentError {
    /// Formats the expected and actual generic argument kinds for diagnostics.
    ///
    /// # Parameters
    ///
    /// * `formatter` — destination for the display representation.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "expected {:?} generic argument, found {:?}",
            self.expected, self.actual
        )
    }
}

impl std::error::Error for GenericArgumentError {}

/// A type expression that cannot provide a requested concrete descriptor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TypeExpressionError {
    /// No concrete reflected type descriptor is available for this parameter.
    ConcreteDescriptorUnavailable {
        /// The name of the generic type parameter without an available
        /// descriptor.
        parameter: &'static str,
    },
}

impl fmt::Display for TypeExpressionError {
    /// Formats the unavailable concrete descriptor and parameter name.
    ///
    /// # Parameters
    ///
    /// * `formatter` — destination for the display representation.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConcreteDescriptorUnavailable { parameter } => {
                write!(
                    formatter,
                    "generic type parameter {parameter} has no reflected descriptor"
                )
            }
        }
    }
}

impl std::error::Error for TypeExpressionError {}
