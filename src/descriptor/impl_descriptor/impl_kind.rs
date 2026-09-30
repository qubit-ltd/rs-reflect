// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! ImplKind metadata and behavior.

use super::impl_descriptor_build_error::ImplDescriptorBuildError;

/// Whether an implementation is inherent or implements a trait.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::ImplKind;
/// let kind = ImplKind::Trait;
/// let description = match kind {
///     ImplKind::Inherent => "inherent implementation",
///     ImplKind::Trait => "trait implementation",
/// };
/// assert_eq!(description, "trait implementation");
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ImplKind {
    /// An inherent implementation block.
    Inherent,
    /// A trait implementation block.
    Trait,
}

impl ImplKind {
    /// Returns the deterministic inherent-before-trait registry rank.
    ///
    /// # Returns
    ///
    /// Returns `0` for inherent impls and `1` for trait impls.
    #[must_use]
    #[inline]
    pub(crate) const fn registry_rank(self) -> u8 {
        match self {
            Self::Inherent => 0,
            Self::Trait => 1,
        }
    }
}

/// Validates that an implementation kind agrees with its trait metadata.
///
/// Inherent implementations must not name a trait, while trait implementations
/// must name one. Callers can use the returned error to reject inconsistent
/// metadata before constructing an implementation definition or instance.
///
/// # Errors
///
/// Returns [`ImplDescriptorBuildError::InherentImplHasTrait`] when an inherent
/// implementation names a trait, or [`ImplDescriptorBuildError::TraitImplMissingTrait`]
/// when a trait implementation does not name one.
#[inline]
pub(in crate::descriptor::impl_descriptor) fn validate_kind(
    kind: ImplKind,
    has_trait: bool,
) -> Result<(), ImplDescriptorBuildError> {
    match (kind, has_trait) {
        (ImplKind::Inherent, true) => Err(ImplDescriptorBuildError::InherentImplHasTrait),
        (ImplKind::Trait, false) => Err(ImplDescriptorBuildError::TraitImplMissingTrait),
        _ => Ok(()),
    }
}
