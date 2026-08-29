//! Parser smoke tests for the three reflection macro entry points.

#![allow(dead_code)]

use std::fs;
use std::process::Command;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use qubit_reflect_derive::Reflect;
use qubit_reflect_derive::reflect;
use qubit_reflect_derive::reflect_impl;

#[derive(Reflect)]
#[reflect(rename = "Record", capabilities(Clone, Default))]
struct Record<T> {
    #[reflect(rename = "identifier", read_only, no_construct, default)]
    id: u64,
    #[reflect(opaque)]
    value: T,
}

#[derive(Reflect)]
enum Event {
    #[reflect(rename = "ready", no_construct)]
    Ready,
    #[reflect(skip)]
    Hidden(#[reflect(default = default_code)] u32),
}

#[derive(Reflect)]
#[reflect(opaque)]
struct Opaque;

#[reflect(external_trait(Send, id = "core.marker.Send"))]
trait Service: Send {
    #[reflect(
        rename = "execute",
        catch_unwind,
        thread_safe,
        specialize(T = String)
    )]
    fn run<T>(&self, value: T);

    #[reflect(no_invoke)]
    fn disabled(&self);
}

#[reflect_impl(specialize(T = String))]
impl<T> Record<T> {
    #[reflect(rename = "current", skip)]
    fn value(&self) -> &T {
        &self.value
    }
}

trait ExternalFormat {
    fn format(&self) -> String;
}

#[reflect_impl(external_trait_id = "example.ExternalFormat")]
impl<T> ExternalFormat for Record<T> {
    fn format(&self) -> String {
        String::new()
    }
}

/// Supplies the field default used by the parser fixture.
fn default_code() -> u32 {
    0
}

#[test]
fn test_macro_parsers_accept_supported_declarations() {
    let _ = Record { id: 1, value: "value" };
    let _ = Event::Ready;
    let _ = Opaque;
}

#[test]
fn test_macro_diagnostic_points_to_the_conflicting_rename_literal() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the system clock should follow the Unix epoch")
        .as_nanos();
    let fixture = std::env::temp_dir().join(format!("qubit-reflect-span-{}-{nonce}", std::process::id()));
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
        r#"use qubit_reflect_derive::Reflect;

#[derive(Reflect)]
struct Duplicate {
#[reflect(rename = "same")]
first: u8,
#[reflect(rename = "same")]
second: u8,
}
"#,
    )
    .expect("the temporary source should be written");

    let output = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .args(["check", "--offline", "--quiet", "--manifest-path"])
        .arg(fixture.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", fixture.join("target"))
        .output()
        .expect("cargo should check the temporary fixture");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    fs::remove_dir_all(&fixture).expect("the temporary fixture should be removed");

    assert!(!output.status.success(), "the duplicate rename should fail");
    assert!(
        stderr.contains("field query name `same` for Rust member `second`"),
        "unexpected compiler diagnostic:\n{stderr}"
    );
    assert!(
        stderr.contains("src/lib.rs:7:20"),
        "the diagnostic should point at the second rename literal:\n{stderr}"
    );
}
