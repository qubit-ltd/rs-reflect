// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0 (the "License");
//    you may not use this file except in compliance with the License.
// =============================================================================

//! Integration checks for optional types in the vNext structural type model.

use qubit_reflect::TypeDescriptor;
use qubit_reflect::descriptor::TypeKind;

#[test]
fn test_optional_types_retain_structural_kind_and_concrete_identity() {
    let optional_u32 = TypeDescriptor::of::<Option<u32>>();
    let optional_text = TypeDescriptor::of::<Option<String>>();

    assert!(matches!(optional_u32.kind(), TypeKind::Optional(_)));
    assert!(matches!(optional_text.kind(), TypeKind::Optional(_)));
    assert_ne!(optional_u32.type_id(), optional_text.type_id());
    assert_eq!(optional_u32.type_id(), std::any::TypeId::of::<Option<u32>>());
}
