// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Generic parameter declarations and their where predicates.

use std::hash::Hash;
use std::hash::Hasher;

use super::generic_parameter_descriptor::GenericParameterDescriptor;
use crate::expression::DiagnosticText;
use crate::expression::PredicateDescriptor;

/// The generic declaration shared by all concrete instances of a reflected
/// item.
///
/// Parameters and predicates preserve source declaration order.  It describes a
/// declaration; it neither synthesizes a runtime type identity for
/// lifetime-only instantiations nor evaluates predicates at runtime.
///
/// # Examples
///
/// ```
/// use qubit_reflect::expression::GenericDefinitionDescriptor;
/// let definition = GenericDefinitionDescriptor::new([], []);
/// assert!(definition.parameters().is_empty());
/// ```
#[derive(Clone, Debug)]
pub struct GenericDefinitionDescriptor {
    /// Lifetime, type, and const parameters in declaration order.
    pub(crate) parameters: Box<[GenericParameterDescriptor]>,
    /// Where-clause predicates in declaration order.
    pub(crate) predicates: Box<[PredicateDescriptor]>,
    /// Optional source-oriented diagnostic text excluded from identity.
    pub(crate) diagnostic: DiagnosticText,
}

impl GenericDefinitionDescriptor {
    /// Creates a generic declaration descriptor.
    ///
    /// # Parameters
    ///
    /// - `parameters`: Generic parameters in source declaration order.
    /// - `predicates`: Where-clause predicates in source declaration order.
    ///
    /// # Returns
    ///
    /// Returns the generic declaration descriptor. Diagnostic text starts
    /// empty.
    pub fn new(
        parameters: impl Into<Box<[GenericParameterDescriptor]>>,
        predicates: impl Into<Box<[PredicateDescriptor]>>,
    ) -> Self {
        Self {
            parameters: parameters.into(),
            predicates: predicates.into(),
            diagnostic: DiagnosticText::default(),
        }
    }

    /// Returns generic parameters in declaration order.
    ///
    /// # Returns
    ///
    /// Returns the declared lifetime, type, and const parameters.
    #[must_use]
    pub fn parameters(&self) -> &[GenericParameterDescriptor] {
        &self.parameters
    }

    /// Returns where-clause predicates in declaration order.
    ///
    /// # Returns
    ///
    /// Returns the structural where-clause predicates.
    #[must_use]
    pub fn predicates(&self) -> &[PredicateDescriptor] {
        &self.predicates
    }

    /// Returns diagnostic text when present.
    ///
    /// # Returns
    ///
    /// Returns source-oriented diagnostic text, or `None` when absent.
    #[must_use]
    pub fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }

    /// Attaches diagnostic text.
    ///
    /// # Parameters
    ///
    /// - `value`: Source-oriented text used only for diagnostics.
    ///
    /// # Returns
    ///
    /// Returns the descriptor with its diagnostic text attached; structural
    /// identity is unchanged.
    #[must_use]
    pub fn with_diagnostic(mut self, value: impl Into<Box<str>>) -> Self {
        self.diagnostic = DiagnosticText::from(value.into());
        self
    }
}

impl PartialEq for GenericDefinitionDescriptor {
    fn eq(&self, other: &Self) -> bool {
        self.parameters == other.parameters && self.predicates == other.predicates
    }
}

impl Eq for GenericDefinitionDescriptor {}

impl Hash for GenericDefinitionDescriptor {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.parameters.hash(state);
        self.predicates.hash(state);
    }
}
