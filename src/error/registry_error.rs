// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Immutable errors reported while aggregating registration fragments.

use std::error::Error;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;
use std::sync::Arc;

use super::registry_error_kind::RegistryErrorKind;
use crate::capability::CapabilityConflict;
use crate::identity::CapabilityId;
use crate::identity::FragmentIdentity;
use crate::registry::fragment::CapabilityTarget;

/// A shareable immutable registry aggregation error.
///
/// # Examples
///
/// ```
/// use qubit_reflect::error::{RegistryError, RegistryErrorKind};
/// use qubit_reflect::identity::FragmentIdentity;
/// let left = FragmentIdentity::new("example", "demo", 1, 1, "type", 10);
/// let right = FragmentIdentity::new("example", "demo", 2, 1, "type", 20);
/// let error = RegistryError::duplicate_fragment(left, right);
/// assert_eq!(error.kind(), RegistryErrorKind::DuplicateFragment);
/// ```
#[must_use]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistryError(
    /// Shared immutable error facts, cloned cheaply across error boundaries.
    Arc<RegistryErrorData>,
);

/// Stores the category and optional registration context behind a registry error.
#[derive(Debug, Eq, PartialEq)]
struct RegistryErrorData {
    /// Stable error category.
    kind: RegistryErrorKind,
    /// First implicated fragment, when present.
    left: Option<FragmentIdentity>,
    /// Second implicated fragment for conflicts, when present.
    right: Option<FragmentIdentity>,
    /// Capability-specific conflict facts, when applicable.
    capability_details: Option<CapabilityConflict>,
    /// Concrete or definition target for a capability conflict.
    capability_target: Option<CapabilityTarget>,
    /// Whether the conflict was caused by an intrinsic capability declaration.
    intrinsic_conflict: bool,
}

impl RegistryError {
    /// Creates an error for two fragments that claim the same registration
    /// identity.
    ///
    /// # Parameters
    ///
    /// - `left`: First fragment claiming the identity.
    /// - `right`: Second fragment claiming the identity.
    ///
    /// # Returns
    ///
    /// Returns an error categorized as a duplicate fragment.
    pub fn duplicate_fragment(left: FragmentIdentity, right: FragmentIdentity) -> Self {
        Self::conflict(RegistryErrorKind::DuplicateFragment, left, right)
    }

    /// Creates an error for fragments that disagree about one identity's
    /// content.
    ///
    /// # Parameters
    ///
    /// - `left`: First fragment with the identity.
    /// - `right`: Fragment whose content disagrees.
    ///
    /// # Returns
    ///
    /// Returns an identity conflict error.
    pub fn identity_conflict(left: FragmentIdentity, right: FragmentIdentity) -> Self {
        Self::conflict(RegistryErrorKind::IdentityConflict, left, right)
    }

    /// Creates an error for incompatible external-trait registrations.
    ///
    /// # Parameters
    ///
    /// - `left`: First fragment registering the external trait.
    /// - `right`: Fragment with the incompatible registration.
    ///
    /// # Returns
    ///
    /// Returns an external trait ID conflict error.
    pub fn external_trait_id_conflict(left: FragmentIdentity, right: FragmentIdentity) -> Self {
        Self::conflict(RegistryErrorKind::ExternalTraitIdConflict, left, right)
    }

    /// Creates an error for incompatible capability registrations.
    ///
    /// # Parameters
    ///
    /// - `left`: First fragment registering the capability.
    /// - `right`: Fragment with the incompatible registration.
    ///
    /// # Returns
    ///
    /// Returns a capability conflict error without detailed conflict facts.
    pub fn capability_conflict(left: FragmentIdentity, right: FragmentIdentity) -> Self {
        Self::conflict(RegistryErrorKind::CapabilityConflict, left, right)
    }

    /// Creates an error for an invalid intrinsic capability declaration.
    ///
    /// # Parameters
    ///
    /// - `fragment`: Fragment containing the invalid intrinsic declaration.
    /// - `conflict`: Complete capability conflict details.
    ///
    /// # Returns
    ///
    /// Returns an intrinsic capability conflict retaining its details.
    pub fn intrinsic_capability_conflict(fragment: FragmentIdentity, conflict: CapabilityConflict) -> Self {
        Self(Arc::new(RegistryErrorData {
            kind: RegistryErrorKind::CapabilityConflict,
            left: Some(fragment),
            right: None,
            capability_details: Some(conflict),
            capability_target: None,
            intrinsic_conflict: true,
        }))
    }

    /// Creates a cross-fragment capability conflict with complete diagnostic
    /// context from the registry builder.
    ///
    /// # Parameters
    ///
    /// - `left`: First conflicting registration fragment.
    /// - `right`: Second conflicting registration fragment.
    /// - `target`: Concrete or definition target involved in the conflict.
    /// - `conflict`: Capability conflict details.
    ///
    /// # Returns
    ///
    /// Returns an error retaining the complete cross-fragment diagnostic
    /// context.
    pub(crate) fn capability_conflict_with_details(
        left: FragmentIdentity,
        right: FragmentIdentity,
        target: CapabilityTarget,
        conflict: CapabilityConflict,
    ) -> Self {
        Self(Arc::new(RegistryErrorData {
            kind: RegistryErrorKind::CapabilityConflict,
            left: Some(left),
            right: Some(right),
            capability_details: Some(conflict),
            capability_target: Some(target),
            intrinsic_conflict: false,
        }))
    }

    /// Creates an intrinsic capability conflict with its concrete registry
    /// target and complete descriptor details.
    ///
    /// # Parameters
    ///
    /// - `fragment`: Fragment containing the intrinsic conflict.
    /// - `target`: Concrete or definition target involved in the conflict.
    /// - `conflict`: Complete capability conflict details.
    ///
    /// # Returns
    ///
    /// Returns an intrinsic capability error retaining its complete context.
    pub(crate) fn intrinsic_capability_conflict_with_target(
        fragment: FragmentIdentity,
        target: CapabilityTarget,
        conflict: CapabilityConflict,
    ) -> Self {
        Self(Arc::new(RegistryErrorData {
            kind: RegistryErrorKind::CapabilityConflict,
            left: Some(fragment),
            right: None,
            capability_details: Some(conflict),
            capability_target: Some(target),
            intrinsic_conflict: true,
        }))
    }

    /// Creates an error when a symbolic generic trait impl cannot resolve one
    /// unique linked trait declaration.
    ///
    /// # Parameters
    ///
    /// - `fragment`: Generic trait impl fragment that could not be linked.
    ///
    /// # Returns
    ///
    /// Returns a trait resolution error identifying the fragment.
    pub fn impl_trait_resolution(fragment: FragmentIdentity) -> Self {
        Self(Arc::new(RegistryErrorData {
            kind: RegistryErrorKind::ImplTraitResolution,
            left: Some(fragment),
            right: None,
            capability_details: None,
            capability_target: None,
            intrinsic_conflict: false,
        }))
    }

    /// Creates an error when the current platform lacks
    /// distributed-registration support.
    ///
    /// # Returns
    ///
    /// Returns an error categorized as an unsupported platform.
    pub fn unsupported_platform() -> Self {
        Self(Arc::new(RegistryErrorData {
            kind: RegistryErrorKind::UnsupportedPlatform,
            left: None,
            right: None,
            capability_details: None,
            capability_target: None,
            intrinsic_conflict: false,
        }))
    }

    /// Returns the stable machine-readable error category.
    ///
    /// # Returns
    ///
    /// Returns the error category.
    #[must_use]
    #[inline]
    pub fn kind(&self) -> RegistryErrorKind {
        let Self(data) = self;
        data.kind
    }

    /// Returns the two conflicting fragments when this error originated from a
    /// conflict.
    ///
    /// # Returns
    ///
    /// Returns both conflicting fragment identities, or `None` when this is not
    /// a two-fragment conflict.
    #[must_use]
    #[inline]
    pub fn conflicting_fragments(&self) -> Option<(&FragmentIdentity, &FragmentIdentity)> {
        let Self(data) = self;
        match (&data.left, &data.right) {
            (Some(left), Some(right)) => Some((left, right)),
            _ => None,
        }
    }

    /// Returns the single implicated fragment for a non-conflict aggregation
    /// error.
    ///
    /// # Returns
    ///
    /// Returns the single implicated fragment, or `None` when there is no
    /// single-fragment context.
    #[must_use]
    #[inline]
    pub fn fragment_identity(&self) -> Option<&FragmentIdentity> {
        let Self(data) = self;
        match (&data.left, &data.right) {
            (Some(fragment), None) => Some(fragment),
            _ => None,
        }
    }

    /// Returns the stable ID involved in a detailed capability conflict.
    ///
    /// # Returns
    ///
    /// Returns the capability ID, or `None` when no detailed capability
    /// conflict is retained.
    #[must_use]
    #[inline]
    pub fn capability_id(&self) -> Option<CapabilityId> {
        let Self(data) = self;
        data.capability_details.as_ref().map(|conflict| *conflict.id())
    }

    /// Returns the complete capability conflict details retained by registry
    /// construction, or `None` when a legacy constructor had no details.
    ///
    /// # Returns
    ///
    /// Returns the retained capability conflict details, or `None` when
    /// unavailable.
    #[must_use]
    #[inline]
    pub fn capability_details(&self) -> Option<&CapabilityConflict> {
        let Self(data) = self;
        data.capability_details.as_ref()
    }

    /// Returns the concrete or definition target involved in a capability
    /// conflict when registry construction supplied it.
    ///
    /// # Returns
    ///
    /// Returns the capability target, or `None` when no target context was
    /// retained.
    #[must_use]
    #[inline]
    pub fn capability_target(&self) -> Option<CapabilityTarget> {
        let Self(data) = self;
        data.capability_target
    }

    /// Returns the complete intrinsic conflict, or `None` for other failures.
    ///
    /// # Returns
    ///
    /// Returns the intrinsic capability conflict, or `None` for non-intrinsic
    /// errors.
    #[must_use]
    pub fn intrinsic_conflict(&self) -> Option<&CapabilityConflict> {
        let Self(data) = self;
        data.intrinsic_conflict.then_some(())?;
        data.capability_details.as_ref()
    }

    /// Creates a conflict error retaining both conflicting registration
    /// fragments.
    ///
    /// # Parameters
    ///
    /// - `kind`: Stable error category for the conflict.
    /// - `left`: First conflicting fragment.
    /// - `right`: Second conflicting fragment.
    ///
    /// # Returns
    ///
    /// Returns a conflict error retaining both fragments.
    fn conflict(kind: RegistryErrorKind, left: FragmentIdentity, right: FragmentIdentity) -> Self {
        Self(Arc::new(RegistryErrorData {
            kind,
            left: Some(left),
            right: Some(right),
            capability_details: None,
            capability_target: None,
            intrinsic_conflict: false,
        }))
    }
}

impl Display for RegistryError {
    /// Formats the category and available registration context.
    ///
    /// # Parameters
    ///
    /// - `formatter`: Formatter receiving the diagnostic.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after formatting, or the formatter error.
    ///
    /// # Errors
    ///
    /// Returns an error reported by the formatter.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
        let Self(data) = self;
        write!(formatter, "reflection registry error: {:?}", data.kind)?;
        if let Some(conflict) = &data.capability_details {
            write!(formatter, " for capability `{}` ({:?})", conflict.id(), conflict.kind(),)?;
        }
        if let Some(target) = data.capability_target {
            write!(formatter, " on {target:?}")?;
        }
        if let Some(left) = &data.left {
            write!(
                formatter,
                " at {}::{}:{}:{} [{}; fingerprint={:#x}]",
                left.declaring_crate(),
                left.module_path(),
                left.line(),
                left.column(),
                left.member_kind(),
                left.content_fingerprint(),
            )?;
        }
        if let Some(right) = &data.right {
            write!(
                formatter,
                " conflicting with {}::{}:{}:{} [{}; fingerprint={:#x}]",
                right.declaring_crate(),
                right.module_path(),
                right.line(),
                right.column(),
                right.member_kind(),
                right.content_fingerprint(),
            )?;
        }
        Ok(())
    }
}

impl Error for RegistryError {
    /// Preserves complete capability conflict details as the underlying cause.
    ///
    /// # Returns
    ///
    /// Returns the retained capability conflict as the source, or `None` when
    /// absent.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.capability_details()
            .map(|conflict| conflict as &dyn Error)
    }
}
