// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Errors returned when building applied trait descriptors.

use std::fmt;

/// An invalid applied trait graph or incomplete external declaration.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::TraitDescriptorBuildError;
/// let error = TraitDescriptorBuildError::ExternalTraitHasUnprovenFacts;
/// assert_eq!(error, TraitDescriptorBuildError::ExternalTraitHasUnprovenFacts);
/// ```
#[must_use]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TraitDescriptorBuildError {
    /// A supertrait resolves to the application currently being built.
    RecursiveSupertrait {
        /// Diagnostic Rust path of the recursive application.
        rust_path: &'static str,
    },
    /// An external incomplete trait attempted to claim unobservable facts.
    ExternalTraitHasUnprovenFacts,
    /// The number of concrete type/const arguments does not match the
    /// definition.
    GenericArgumentCount {
        /// Number of runtime identity arguments required by the definition.
        expected: usize,
        /// Number of arguments supplied by the applied descriptor.
        actual: usize,
    },
    /// An argument kind does not match its type or const parameter.
    GenericArgumentKind {
        /// Zero-based runtime identity argument index.
        index: usize,
    },
    /// An argument still contains a symbolic type or const parameter.
    NonConcreteGenericArgument {
        /// Zero-based runtime identity argument index.
        index: usize,
    },
    /// An associated-type argument is unknown, duplicated, or non-concrete.
    InvalidAssociatedTypeArgument,
    /// A method declaration belongs to another trait or an impl.
    ForeignMethod,
}

impl fmt::Display for TraitDescriptorBuildError {
    /// Formats a stable diagnostic message.
    ///
    /// # Parameters
    ///
    /// - `formatter`: Formatter receiving the diagnostic message.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after writing the message.
    ///
    /// # Errors
    ///
    /// Returns the formatter error when writing fails.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RecursiveSupertrait { rust_path } => {
                write!(formatter, "recursive supertrait application: {rust_path}")
            }
            Self::ExternalTraitHasUnprovenFacts => {
                formatter.write_str("an external incomplete trait cannot claim supertraits or associated items")
            }
            Self::GenericArgumentCount { expected, actual } => write!(
                formatter,
                "trait application requires {expected} concrete arguments but received {actual}"
            ),
            Self::GenericArgumentKind { index } => {
                write!(formatter, "trait argument {index} has the wrong generic kind")
            }
            Self::NonConcreteGenericArgument { index } => {
                write!(formatter, "trait argument {index} is not concrete")
            }
            Self::InvalidAssociatedTypeArgument => {
                formatter.write_str("an associated-type argument must name one declared item and have a concrete value")
            }
            Self::ForeignMethod => formatter.write_str("applied trait contains a foreign method declaration"),
        }
    }
}

impl std::error::Error for TraitDescriptorBuildError {}
