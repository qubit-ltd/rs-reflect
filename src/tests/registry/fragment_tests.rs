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
use crate::registry::fragment::RuntimeIdentity;

/// Verifies payload categories and runtime identities agree for a type root.
#[test]
fn test_fragment_payload_reports_matching_kind_and_identity() {
    let descriptor = TypeDescriptor::of::<u8>();
    let payload = FragmentPayload::Type(descriptor);
    assert_eq!(payload.kind(), FragmentKind::Type);
    assert_eq!(payload.runtime_identity(), RuntimeIdentity::Type(descriptor.type_id()));
}
