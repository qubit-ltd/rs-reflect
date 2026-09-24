// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Tests materialized registration payload inspection.

use crate::TypeDescriptor;
use crate::registry::fragment::FragmentKind;
use crate::registry::fragment::FragmentPayload;
use crate::registry::fragment::RegistrationFragment;
use crate::registry::fragment::RuntimeIdentity;
use crate::registry::fragment::StaticFragmentIdentity;

/// Returns the runtime identity used by the registration-fragment fixture.
fn fixture_runtime_identity() -> RuntimeIdentity {
    RuntimeIdentity::Type(TypeDescriptor::of::<u16>().type_id())
}

/// Builds the payload used by the registration-fragment fixture.
fn fixture_payload() -> FragmentPayload {
    FragmentPayload::Type(TypeDescriptor::of::<u16>())
}

/// Verifies payload categories and runtime identities agree for a type root.
#[test]
fn test_fragment_payload_reports_matching_kind_and_identity() {
    let descriptor = TypeDescriptor::of::<u8>();
    let payload = FragmentPayload::Type(descriptor);
    assert_eq!(payload.kind(), FragmentKind::Type);
    assert_eq!(payload.runtime_identity(), RuntimeIdentity::Type(descriptor.type_id()));
}

/// Verifies a static fragment materializes all deferred facts without changing
/// its declared category or source identity.
#[test]
fn test_registration_fragment_materializes_declared_facts() {
    let fragment = RegistrationFragment::new(
        FragmentKind::Type,
        StaticFragmentIdentity::new("fixture", "fixture::module", 7, 11, "type", 41),
        fixture_runtime_identity,
        fixture_payload,
    );

    assert_eq!(fragment.kind(), FragmentKind::Type);
    assert_eq!(fragment.identity().declaring_crate(), "fixture");
    assert_eq!(fragment.identity().module_path(), "fixture::module");
    assert_eq!(fragment.identity().line(), 7);
    assert_eq!(fragment.identity().column(), 11);
    assert_eq!(fragment.identity().member_kind(), "type");
    assert_eq!(fragment.identity().content_fingerprint(), 41);
    assert_eq!(fragment.target_identity(), fixture_runtime_identity());
    assert_eq!(fragment.build().kind(), FragmentKind::Type);
}
