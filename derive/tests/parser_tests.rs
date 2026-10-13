// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Parser smoke tests for the three reflection macro entry points.

#![allow(dead_code)]

use std::env;
use std::fs;
use std::process::Command;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use qubit_reflect_derive::Reflect;
use qubit_reflect_derive::ReflectImpl;

#[Reflect]
#[reflect(capabilities(Clone, Default))]
struct Record<T: Clone + Default> {
    id: u64,
    #[reflect(opaque)]
    value: T,
}

#[Reflect]
enum Event {
    Ready,
    #[reflect(skip)]
    Hidden,
}

#[Reflect]
#[reflect(opaque)]
struct Opaque;

#[Reflect(external_trait(Send, id = "core.marker.Send"))]
trait Service: Send {
    #[reflect(catch_unwind)]
    fn run<T>(&self, value: T);

    #[reflect(no_invoke)]
    fn disabled(&self);
}

#[ReflectImpl]
impl<T: Clone + Default> Record<T> {
    #[reflect(skip)]
    fn value(&self) -> &T {
        &self.value
    }
}

#[Reflect]
trait ExternalFormat {
    fn format(&self) -> String;
}

#[ReflectImpl]
impl<T: Clone + Default> ExternalFormat for Record<T> {
    fn format(&self) -> String {
        String::new()
    }
}

#[test]
fn test_macro_parsers_accept_supported_declarations() {
    let _ = Record { id: 1, value: "value" };
    let _ = Event::Ready;
    let _ = Opaque;
}

#[test]
fn test_macro_diagnostic_rejects_a_removed_rename_option() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the system clock should follow the Unix epoch")
        .as_nanos();
    let fixture = env::temp_dir().join(format!("qubit-reflect-span-{}-{nonce}", std::process::id()));
    let source_dir = fixture.join("src");
    fs::create_dir_all(&source_dir).expect("the temporary fixture should be created");
    let dependency_path = env!("CARGO_MANIFEST_DIR").replace('\\', "\\\\").replace('"', "\\\"");
    fs::write(
        fixture.join("Cargo.toml"),
        format!(
            r#"[package]
name = "qubit-reflect-span-fixture"
version = "0.0.0"
edition = "2024"

[dependencies]
qubit-reflect-derive = {{ path = "{dependency_path}" }}
"#,
        ),
    )
    .expect("the temporary manifest should be written");
    fs::write(
        source_dir.join("lib.rs"),
        concat!(
            "use ",
            "qubit_reflect_derive",
            r#"::Reflect;

#[Reflect]
struct Duplicate {
#[reflect(rename = "same")]
first: u8,
#[reflect(rename = "same")]
second: u8,
}
"#
        ),
    )
    .expect("the temporary source should be written");

    let output = Command::new(env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .args(["check", "--offline", "--quiet", "--manifest-path"])
        .arg(fixture.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", fixture.join("target"))
        .output()
        .expect("cargo should check the temporary fixture");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    fs::remove_dir_all(&fixture).expect("the temporary fixture should be removed");

    assert!(!output.status.success(), "the removed rename option should fail");
    assert!(
        stderr.contains("`rename` was removed"),
        "unexpected compiler diagnostic:\n{stderr}"
    );
}
