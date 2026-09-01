// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow explicit-imports
//! Integration tests for type identity and base error APIs.
use std::any::TypeId;
use std::str::FromStr;

use qubit_reflect::error::RegistryError;
use qubit_reflect::error::RegistryErrorKind;
use qubit_reflect::error::TypeMismatch;
use qubit_reflect::identity::CapabilityId;
use qubit_reflect::identity::ExternalTraitId;
use qubit_reflect::identity::FragmentIdentity;
use qubit_reflect::identity::MemberId;
use qubit_reflect::identity::Visibility;
use qubit_reflect::identity::VisibilityKind;

/// Verifies namespaced identifiers accept valid external names.
#[test]
fn test_namespaced_ids_accept_ascii_identifier_segments() {
    let capability = CapabilityId::new("example.fixture.clone")
        .expect("valid capability ID");
    let external = ExternalTraitId::from_str("example.fixture.display")
        .expect("valid trait ID");

    assert_eq!(capability.as_str(), "example.fixture.clone");
    assert_eq!(capability.as_ref(), "example.fixture.clone");
    assert_eq!(capability.to_string(), "example.fixture.clone");
    assert_eq!(
        CapabilityId::from_str("example.fixture.clone")
            .expect("capability parses"),
        capability,
    );
    assert!(CapabilityId::new("Example.fixture_2.Clone3").is_ok());
    assert_eq!(external.as_str(), "example.fixture.display");
    assert_eq!(external.as_ref(), "example.fixture.display");
    assert_eq!(external.to_string(), "example.fixture.display");
}

/// Verifies namespaced identifiers reject non-ASCII and malformed segments.
#[test]
fn test_namespaced_ids_reject_non_ascii_and_malformed_segments() {
    assert!(CapabilityId::new("例子.fixture.clone").is_err());
    assert!(CapabilityId::new("example.fixture-clone").is_err());
    assert!(CapabilityId::new("example.2fixture.clone").is_err());
    assert!(CapabilityId::new("example..clone").is_err());
    assert!(CapabilityId::new(".example.clone").is_err());
    assert!(CapabilityId::new("example.clone.").is_err());
}

/// Verifies external capability IDs cannot claim the library-reserved
/// namespace.
#[test]
fn test_capability_id_rejects_reserved_namespace() {
    assert!(CapabilityId::new("qubit.reflect").is_err());
    assert!(CapabilityId::new("qubit.reflect.clone").is_err());
    assert!(ExternalTraitId::new("qubit.reflect.external").is_err());
}

/// Verifies member identities combine their declaring identity, category,
/// index, and fragment.
#[test]
fn test_member_id_is_a_composite_identity() {
    let fragment = FragmentIdentity::new(
        "example_crate",
        "example::model",
        12,
        5,
        "field",
        42,
    );
    let member =
        MemberId::new("example::model::User", "field", 0, fragment.clone());

    assert_eq!(member.declaring_identity(), "example::model::User");
    assert_eq!(member.kind(), "field");
    assert_eq!(member.index(), 0);
    assert_eq!(member.fragment(), &fragment);
}

/// Verifies source visibilities normalize to their stable categories.
#[test]
fn test_visibility_normalizes_source_forms() {
    assert_eq!(
        Visibility::from_source("pub").kind(),
        VisibilityKind::Public
    );
    assert_eq!(
        Visibility::from_source("pub(crate)").kind(),
        VisibilityKind::Crate
    );
    assert_eq!(
        Visibility::from_source("pub(super)").kind(),
        VisibilityKind::Super
    );
    assert_eq!(
        Visibility::from_source("pub(self)").kind(),
        VisibilityKind::Private
    );
}

/// Verifies restricted visibilities retain their source path for diagnostics.
#[test]
fn test_visibility_preserves_restricted_path() {
    let visibility = Visibility::from_source("pub(in crate::model)");

    assert_eq!(visibility.kind(), VisibilityKind::Restricted);
    assert_eq!(visibility.restricted_path(), Some("crate::model"));
}

/// Verifies an empty restricted path is not treated as a valid restriction.
#[test]
fn test_visibility_rejects_empty_restricted_path() {
    let visibility = Visibility::from_source("pub(in )");

    assert_eq!(visibility.kind(), VisibilityKind::Private);
    assert_eq!(visibility.restricted_path(), None);
}

/// Verifies registry errors retain their machine-readable kind after
/// inexpensive cloning.
#[test]
fn test_registry_error_clone_preserves_kind() {
    let left =
        FragmentIdentity::new("example", "example::one", 1, 1, "impl", 1);
    let right =
        FragmentIdentity::new("example", "example::two", 2, 1, "impl", 2);
    let error = RegistryError::duplicate_fragment(left.clone(), right.clone());
    let cloned = error.clone();

    assert_eq!(error.kind(), cloned.kind());
    let (actual_left, actual_right) = error
        .conflicting_fragments()
        .expect("duplicate error should retain both fragments");
    assert_eq!(actual_left, &left);
    assert_eq!(actual_right, &right);
    assert!(error.fragment_identity().is_none());
    assert_eq!(
        error.to_string(),
        "reflection registry error: DuplicateFragment"
    );
}

/// Verifies every registry error constructor retains its category and
/// applicable fragment context.
#[test]
fn test_registry_error_constructors_preserve_context() {
    let left =
        FragmentIdentity::new("example", "example::left", 1, 1, "impl", 1);
    let right =
        FragmentIdentity::new("example", "example::right", 2, 1, "impl", 2);
    let conflicts = [
        (
            RegistryError::identity_conflict(left.clone(), right.clone()),
            RegistryErrorKind::IdentityConflict,
        ),
        (
            RegistryError::external_trait_id_conflict(
                left.clone(),
                right.clone(),
            ),
            RegistryErrorKind::ExternalTraitIdConflict,
        ),
        (
            RegistryError::capability_conflict(left.clone(), right.clone()),
            RegistryErrorKind::CapabilityConflict,
        ),
    ];

    for (error, expected_kind) in conflicts {
        assert_eq!(error.kind(), expected_kind);
        assert_eq!(error.conflicting_fragments(), Some((&left, &right)));
        assert!(error.fragment_identity().is_none());
    }

    let resolution = RegistryError::impl_trait_resolution(left.clone());
    assert_eq!(resolution.kind(), RegistryErrorKind::ImplTraitResolution);
    assert_eq!(resolution.fragment_identity(), Some(&left));
    assert!(resolution.conflicting_fragments().is_none());

    let unsupported = RegistryError::unsupported_platform();
    assert_eq!(unsupported.kind(), RegistryErrorKind::UnsupportedPlatform);
    assert!(unsupported.fragment_identity().is_none());
    assert!(unsupported.conflicting_fragments().is_none());
}

/// Verifies type mismatch diagnostics retain exact IDs independently from
/// optional human-readable names.
#[test]
fn test_type_mismatch_preserves_ids_and_optional_names() {
    let unnamed = TypeMismatch::new(TypeId::of::<u8>(), TypeId::of::<u16>());
    assert_eq!(unnamed.expected(), TypeId::of::<u8>());
    assert_eq!(unnamed.actual(), TypeId::of::<u16>());
    assert_eq!(unnamed.expected_name(), None);
    assert_eq!(unnamed.actual_name(), None);
    assert_eq!(
        unnamed.to_string(),
        "dynamic value type did not match the expected type"
    );

    let named = unnamed.with_diagnostic_names("u8", "u16");
    assert_eq!(named.expected_name(), Some("u8"));
    assert_eq!(named.actual_name(), Some("u16"));
}
