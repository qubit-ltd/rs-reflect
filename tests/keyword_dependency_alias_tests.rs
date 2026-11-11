// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![cfg(feature = "derive")]

use std::path::PathBuf;
use std::process::Command;

#[test]
fn test_keyword_runtime_dependency_alias_is_supported() {
    run_fixture("keyword-runtime");
}

#[test]
fn test_keyword_direct_derive_dependency_alias_is_supported() {
    run_fixture("keyword-derive");
}

fn run_fixture(name: &str) {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest = manifest_dir.join("tests/fixtures").join(name).join("Cargo.toml");
    let target_dir = manifest_dir.join("target/keyword-dependency-alias").join(name);
    let output = Command::new("cargo")
        .arg("run")
        .arg("--locked")
        .arg("--manifest-path")
        .arg(&manifest)
        .env("CARGO_TARGET_DIR", target_dir)
        .output()
        .expect("cargo must run the keyword dependency alias consumer");
    assert!(
        output.status.success(),
        "keyword alias consumer {name} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
