// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Guards the shared derive/runtime protocol handshake.

use std::fs;
use std::path::Path;

/// Ensures configured attributes enter the codegen v4 derive support path.
#[test]
fn test_configured_reflection_uses_codegen_v4_support() {
    let derive_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let pipeline = fs::read_to_string(derive_root.join("src/configure/pipeline.rs"))
        .expect("derive pipeline source should be readable");
    let runtime_root = derive_root.join("../src/private/codegen_v4");
    let support =
        fs::read_to_string(runtime_root.join("macro_support.rs")).expect("v4 macro support source should be readable");

    assert!(
        pipeline.contains("__private::codegen_v4::macro_support::ConfiguredReflection"),
        "configured derive must enter the v4 support path",
    );
    assert!(
        !pipeline.contains("__private::codegen_v3::macro_support"),
        "configured derive must not use v3 macro support",
    );
    assert!(
        support.contains("qubit_reflect_derive::ConfiguredReflection"),
        "v4 macro support must re-export the configured derive entry",
    );
}
