// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::error::RegistryError;
use qubit_reflect::error::RegistryErrorKind;
use qubit_reflect::identity::FragmentIdentity;

fn fragment(module: &'static str, line: u32) -> FragmentIdentity {
    FragmentIdentity::new(
        "registry-error-test",
        module,
        line,
        3,
        "method-binding",
        u64::from(line),
    )
}

#[test]
fn ambiguous_method_binding_error_keeps_selector_count_and_location() {
    let declaration = fragment("crate::service", 41);
    let error = RegistryError::method_binding_ambiguous(declaration.clone(), "render", 2);

    assert_eq!(error.kind(), RegistryErrorKind::MethodBindingAmbiguous);
    assert_eq!(error.method_binding_name(), Some("render"));
    assert_eq!(error.method_binding_matches(), Some(2));
    assert_eq!(error.fragment_identity(), Some(&declaration));
    assert_eq!(error.conflicting_fragments(), None);
    assert_eq!(
        error.to_string(),
        "reflection registry error: MethodBindingAmbiguous for method binding `render` (2 matches) at registry-error-test::crate::service:41:3 [method-binding; fingerprint=0x29]",
    );
}

#[test]
fn duplicate_method_binding_error_keeps_both_declarations_and_selector() {
    let first = fragment("crate::first", 17);
    let second = fragment("crate::second", 29);
    let error = RegistryError::method_binding_conflict(first.clone(), second.clone(), "render");

    assert_eq!(error.kind(), RegistryErrorKind::MethodBindingConflict);
    assert_eq!(error.method_binding_name(), Some("render"));
    assert_eq!(error.method_binding_matches(), Some(2));
    assert_eq!(error.fragment_identity(), None);
    assert_eq!(error.conflicting_fragments(), Some((&first, &second)));
    assert!(std::error::Error::source(&error).is_none());
}
