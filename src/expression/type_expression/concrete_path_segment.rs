// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Structural representations of Rust type expressions.

//! ConcretePathSegment structure and operations.

use crate::expression::GenericArgument;

/// One concrete path segment and the generic arguments written on it.
///
/// # Examples
///
/// ```
/// use qubit_reflect::expression::ConcretePathSegment;
/// let segment = ConcretePathSegment::new("Vec", []);
/// assert_eq!(segment.name(), "Vec");
/// assert!(segment.arguments().is_empty());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConcretePathSegment {
    /// Identifier for this path component.
    name: Box<str>,
    /// Generic arguments written on this exact component.
    arguments: Box<[GenericArgument]>,
}

impl ConcretePathSegment {
    /// Creates a structural concrete path segment.
    ///
    /// # Parameters
    ///
    /// - `name`: Path segment text; containing paths validate that it is
    ///   non-empty.
    /// - `arguments`: Generic arguments written on this segment.
    ///
    /// # Returns
    ///
    /// Returns the structural path segment.
    pub fn new(name: impl Into<Box<str>>, arguments: impl Into<Box<[GenericArgument]>>) -> Self {
        Self {
            name: name.into(),
            arguments: arguments.into(),
        }
    }

    /// Returns this path segment's identifier.
    ///
    /// # Returns
    ///
    /// Returns the segment name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns generic arguments attached to this exact segment.
    ///
    /// # Returns
    ///
    /// Returns the arguments retained at this segment.
    #[must_use]
    pub fn arguments(&self) -> &[GenericArgument] {
        &self.arguments
    }
}
