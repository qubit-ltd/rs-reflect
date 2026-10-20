// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use renamed_reflect::ReflectionContext;
use renamed_reflect::TypeDescriptor;

#[test]
fn same_named_types_from_two_crate_versions_keep_distinct_identities() {
    let context = ReflectionContext::builder()
        .include_linked()
        .build()
        .expect("both versioned dependency fragments register successfully");
    let v1 = TypeDescriptor::of_in::<version_v1::SameName>(&context);
    let v2 = TypeDescriptor::of_in::<version_v2::SameName>(&context);

    assert_eq!(v1.short_name(), v2.short_name());
    assert_ne!(v1.type_id(), v2.type_id());
    assert_ne!(
        v1.definition().expect("v1 retains its definition").id(),
        v2.definition().expect("v2 retains its definition").id(),
    );

    let by_id_v1 = context
        .find_type_by_id(v1.type_id())
        .expect("v1 identity lookup succeeds")
        .expect("v1 remains registered");
    let by_id_v2 = context
        .find_type_by_id(v2.type_id())
        .expect("v2 identity lookup succeeds")
        .expect("v2 remains registered");
    assert_eq!(by_id_v1.type_id(), v1.type_id());
    assert_eq!(by_id_v2.type_id(), v2.type_id());

    let by_name = context
        .lookup_type_by_short_name(v1.short_name())
        .expect("name lookup succeeds");
    assert!(by_name.is_ambiguous());
    assert_eq!(by_name.len(), 2);
    assert!(
        by_name
            .candidates()
            .iter()
            .any(|candidate| candidate.type_id() == v1.type_id())
    );
    assert!(
        by_name
            .candidates()
            .iter()
            .any(|candidate| candidate.type_id() == v2.type_id())
    );
}
