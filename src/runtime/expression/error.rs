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
    expected: GenericArgumentKind,
    actual: GenericArgumentKind,
}

impl GenericArgumentError {
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
