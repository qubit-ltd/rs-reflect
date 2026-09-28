// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Errors from constructing method instances.

use std::fmt;

/// An inconsistent method implementation source or invocation capability.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::MethodInstanceBuildError;
/// let error = MethodInstanceBuildError::RequiredMethodHasAdapter;
/// assert_eq!(error, MethodInstanceBuildError::RequiredMethodHasAdapter);
/// ```
#[must_use]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MethodInstanceBuildError {
    /// An inherent instance does not reference an impl-owned declaration.
    DeclaredMethodNotOwnedByImpl,
    /// A trait instance does not reference a trait-owned declaration.
    TraitMethodNotOwnedByTrait,
    /// A required method incorrectly advertises an invocation adapter.
    RequiredMethodHasAdapter,
    /// An overridden instance does not name its concrete impl method.
    OverriddenMethodMissingImplementation,
    /// A non-overridden instance names a concrete impl method.
    UnexpectedImplementationMethod,
    /// An available adapter and unavailable reasons were supplied together.
    AdapterHasUnavailableReasons,
    /// No adapter and no structured unavailable reason were supplied.
    UnavailableMethodMissingReasons,
}

impl fmt::Display for MethodInstanceBuildError {
    /// Formats a stable diagnostic message.
    ///
    /// # Parameters
    ///
    /// - `formatter`: Destination for the diagnostic message.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after writing the message.
    ///
    /// # Errors
    ///
    /// Returns the formatter error if the destination rejects the message.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeclaredMethodNotOwnedByImpl => {
                formatter.write_str("a declared inherent method must be owned by an impl")
            }
            Self::TraitMethodNotOwnedByTrait => formatter.write_str("a trait method instance must be owned by a trait"),
            Self::RequiredMethodHasAdapter => {
                formatter.write_str("a required method cannot have an invocation adapter")
            }
            Self::OverriddenMethodMissingImplementation => {
                formatter.write_str("an overridden method must name its impl method")
            }
            Self::UnexpectedImplementationMethod => {
                formatter.write_str("only an overridden method can name an impl method")
            }
            Self::AdapterHasUnavailableReasons => {
                formatter.write_str("an available invocation adapter cannot have unavailable reasons")
            }
            Self::UnavailableMethodMissingReasons => {
                formatter.write_str("an unavailable method must provide a structured reason")
            }
        }
    }
}

impl std::error::Error for MethodInstanceBuildError {}
