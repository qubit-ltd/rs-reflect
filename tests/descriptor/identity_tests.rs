//! Integration tests for type identity and base error APIs.

use qubit_reflect::error::RegistryError;
use qubit_reflect::identity::{
    CapabilityId, ExternalTraitId, FragmentIdentity, MemberId, Visibility, VisibilityKind,
};

/// Verifies namespaced identifiers accept valid external names and reject malformed or reserved ones.
#[test]
fn test_namespaced_ids_validate_segments_and_reserved_namespace() {
    assert!(CapabilityId::new("example.fixture.clone").is_ok());
    assert!(ExternalTraitId::new("example.fixture.display").is_ok());
    assert!(CapabilityId::new("example..clone").is_err());
    assert!(CapabilityId::new(".example.clone").is_err());
    assert!(CapabilityId::new("example.clone.").is_err());
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
    assert_eq!(Visibility::Public, Visibility::from_source("pub"));
    assert_eq!(Visibility::Crate, Visibility::from_source("pub(crate)"));
    assert_eq!(Visibility::Super, Visibility::from_source("pub(super)"));
    assert_eq!(
        Visibility::Restricted,
        Visibility::from_source("pub(in crate::model)")
    );
    assert_eq!(Visibility::Private, Visibility::from_source("pub(self)"));
    assert_eq!(Visibility::Private.kind(), VisibilityKind::Private);
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
