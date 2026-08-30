//! Compile-time contracts for public reflection macros.

#![cfg(feature = "derive")]

#[test]
fn reflection_macro_ui_contracts() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/ui/pass/*.rs");
    cases.compile_fail("tests/ui/fail/*.rs");
}
