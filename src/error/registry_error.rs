// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Immutable errors reported while aggregating registration fragments.

use std::sync::Arc;

use super::registry_error_kind::RegistryErrorKind;
use crate::capability::CapabilityConflict;
use crate::identity::CapabilityId;
use crate::identity::FragmentIdentity;
use crate::registry::fragment::CapabilityTarget;

/// A shareable immutable registry aggregation error.
#[must_use]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistryError(Arc<RegistryErrorData>);

#[derive(Debug, Eq, PartialEq)]
struct RegistryErrorData {
    kind: RegistryErrorKind,
    left: Option<FragmentIdentity>,
    right: Option<FragmentIdentity>,
    capability_details: Option<CapabilityConflict>,
    capability_target: Option<CapabilityTarget>,
    intrinsic_conflict: bool,
}

impl RegistryError {
    /// Creates an error for two fragments that claim the same registration
    /// identity.
    pub fn duplicate_fragment(left: FragmentIdentity, right: FragmentIdentity) -> Self {
        Self::conflict(RegistryErrorKind::DuplicateFragment, left, right)
    }

    /// Creates an error for fragments that disagree about one identity's
    /// content.
    pub fn identity_conflict(left: FragmentIdentity, right: FragmentIdentity) -> Self {
        Self::conflict(RegistryErrorKind::IdentityConflict, left, right)
    }

    /// Creates an error for incompatible external-trait registrations.
    pub fn external_trait_id_conflict(left: FragmentIdentity, right: FragmentIdentity) -> Self {
        Self::conflict(RegistryErrorKind::ExternalTraitIdConflict, left, right)
    }

    /// Creates an error for incompatible capability registrations.
    pub fn capability_conflict(left: FragmentIdentity, right: FragmentIdentity) -> Self {
        Self::conflict(RegistryErrorKind::CapabilityConflict, left, right)
    }

    /// Creates an error for an invalid intrinsic capability declaration.
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
    #[must_use]
    #[inline]
    pub fn kind(&self) -> RegistryErrorKind {
        let Self(data) = self;
        data.kind
    }

    /// Returns the two conflicting fragments when this error originated from a
    /// conflict.
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
    #[must_use]
    #[inline]
    pub fn capability_id(&self) -> Option<CapabilityId> {
        let Self(data) = self;
        data.capability_details.as_ref().map(|conflict| *conflict.id())
    }

    /// Returns the complete capability conflict details retained by registry
    /// construction, or `None` when a legacy constructor had no details.
    #[must_use]
    #[inline]
    pub fn capability_details(&self) -> Option<&CapabilityConflict> {
        let Self(data) = self;
        data.capability_details.as_ref()
    }

    /// Returns the concrete or definition target involved in a capability
    /// conflict when registry construction supplied it.
    #[must_use]
    #[inline]
    pub fn capability_target(&self) -> Option<CapabilityTarget> {
        let Self(data) = self;
        data.capability_target
    }

    /// Returns the complete intrinsic conflict, or `None` for other failures.
    #[must_use]
    pub fn intrinsic_conflict(&self) -> Option<&CapabilityConflict> {
        let Self(data) = self;
        data.intrinsic_conflict.then_some(())?;
        data.capability_details.as_ref()
    }

    /// Creates a conflict error retaining both conflicting registration
    /// fragments.
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

impl std::fmt::Display for RegistryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

impl std::error::Error for RegistryError {
    /// Preserves complete capability conflict details as the underlying cause.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.capability_details()
            .map(|conflict| conflict as &dyn std::error::Error)
    }
}
