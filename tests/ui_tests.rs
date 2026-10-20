// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Compile-time contracts for public reflection macros.

#![cfg(feature = "derive")]

#[test]
fn test_reflection_macro_ui_contracts() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/ui/pass/*.rs");
    cases.compile_fail("tests/ui/fail/*.rs");
}

#[test]
fn test_capability_handles_cannot_escape_their_context_lifetime() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/fail/capability_context_escape.rs");
}

#[test]
fn test_removed_v3_public_names_stay_unavailable() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/fail/removed_v3_facade_names.rs");
    cases.compile_fail("tests/ui/fail/removed_public_id_error.rs");
}
