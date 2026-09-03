// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Guards the versioned runtime protocol used by generated reflection code.

use std::fs;
use std::path::Path;

/// Recursively visits Rust source files below `directory`.
fn visit_rust_files(directory: &Path, visit: &mut impl FnMut(&Path)) {
    for entry in fs::read_dir(directory).expect("derive source directory should be readable") {
        let entry = entry.expect("derive source entry should be readable");
        let path = entry.path();
        if path.is_dir() {
            visit_rust_files(&path, visit);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            visit(&path);
        }
    }
}

/// Ensures every generated facade path enters the versioned codegen protocol.
#[test]
fn test_generated_facade_paths_use_codegen_v2() {
    let expand_directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/expand");
    let mut violations = Vec::new();

    visit_rust_files(&expand_directory, &mut |path| {
        let source = fs::read_to_string(path).expect("derive source file should be readable");
        for (index, line) in source.lines().enumerate() {
            let mut remainder = line;
            while let Some(position) = remainder.find("#facade") {
                let candidate = &remainder[position..];
                if !candidate.starts_with("#facade::__private::codegen_v2") {
                    violations.push(format!("{}:{}: {}", path.display(), index + 1, line.trim()));
                }
                remainder = &candidate["#facade".len()..];
            }
        }
    });

    assert!(
        violations.is_empty(),
        "generated facade paths must use __private::codegen_v2:\n{}",
        violations.join("\n"),
    );
}
