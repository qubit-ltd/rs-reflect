// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! ImplDescriptorBuildError metadata and behavior.

use std::fmt;

/// An invalid impl definition or concrete application.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::ImplDescriptorBuildError;
/// let error = ImplDescriptorBuildError::TraitImplMissingTrait;
/// assert_eq!(error, ImplDescriptorBuildError::TraitImplMissingTrait);
/// ```
#[must_use]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ImplDescriptorBuildError {
    /// An inherent impl attempted to name an implemented trait.
    InherentImplHasTrait,
    /// A trait impl omitted its trait link.
    TraitImplMissingTrait,
    /// Concrete arguments do not match the impl definition.
    GenericArgumentsDoNotMatchDefinition,
    /// The applied trait does not originate from the definition's trait.
    ImplementedTraitDefinitionMismatch,
    /// A method or associated binding belongs to another descriptor graph.
    ForeignMember,
}

impl fmt::Display for ImplDescriptorBuildError {
    /// Formats a stable diagnostic message.
    ///
    /// # Parameters
    ///
    /// - `formatter`: Formatter receiving the diagnostic message.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after writing the message, or the formatter error.
    ///
    /// # Errors
    ///
    /// Returns the error reported by `formatter`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InherentImplHasTrait => formatter.write_str("an inherent impl cannot name a trait"),
            Self::TraitImplMissingTrait => formatter.write_str("a trait impl must name a trait"),
            Self::GenericArgumentsDoNotMatchDefinition => {
                formatter.write_str("concrete impl arguments do not match the definition")
            }
            Self::ImplementedTraitDefinitionMismatch => {
                formatter.write_str("applied trait does not match the impl definition")
            }
            Self::ForeignMember => formatter.write_str("impl descriptor contains a foreign member"),
        }
    }
}

impl std::error::Error for ImplDescriptorBuildError {}
