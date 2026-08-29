//! Integration tests for type identity and base error APIs.

use qubit_reflect::error::RegistryError;
use qubit_reflect::identity::{
    CapabilityId, ExternalTraitId, FragmentIdentity, MemberId, Visibility, VisibilityKind,
};

/// Verifies namespaced identifiers accept valid external names.
#[test]
fn test_namespaced_ids_accept_ascii_identifier_segments() {
    assert!(CapabilityId::new("example.fixture.clone").is_ok());
    assert!(CapabilityId::new("Example.fixture_2.Clone3").is_ok());
    assert!(ExternalTraitId::new("example.fixture.display").is_ok());
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

/// Verifies external capability IDs cannot claim the library-reserved namespace.
#[test]
fn test_capability_id_rejects_reserved_namespace() {
    assert!(CapabilityId::new("qubit.reflect").is_err());
    assert!(CapabilityId::new("qubit.reflect.clone").is_err());
    assert!(ExternalTraitId::new("qubit.reflect.external").is_err());
}

/// Verifies member identities combine their declaring identity, category, index, and fragment.
#[test]
fn test_member_id_is_a_composite_identity() {
    let fragment = FragmentIdentity::new("example_crate", "example::model", 12, 5, "field", 42);
    let member = MemberId::new("example::model::User", "field", 0, fragment.clone());

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

/// Verifies registry errors retain their machine-readable kind after inexpensive cloning.
#[test]
fn test_registry_error_clone_preserves_kind() {
    let left = FragmentIdentity::new("example", "example::one", 1, 1, "impl", 1);
    let right = FragmentIdentity::new("example", "example::two", 2, 1, "impl", 2);
    let error = RegistryError::duplicate_fragment(left, right);
    let cloned = error.clone();

    assert_eq!(error.kind(), cloned.kind());
}
