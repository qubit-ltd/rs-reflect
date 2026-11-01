// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

struct Snippet {
    file: &'static str,
    heading: &'static str,
    prefix_heading: Option<&'static str>,
    derive: bool,
    main: &'static str,
}

const SNIPPETS: &[Snippet] = &[
    Snippet {
        file: "README.md",
        heading: "## Quick start",
        prefix_heading: None,
        derive: false,
        main: "fn main() { let mut customer = Customer { email: String::new(), credit_limit_cents: 10_000 }; update(&mut customer).expect(\"update customer\"); assert_eq!(customer.email, \"ada@corp.example\"); assert_eq!(customer.credit_limit_cents, 75_000); }",
    },
    Snippet {
        file: "README.zh_CN.md",
        heading: "## 快速开始",
        prefix_heading: None,
        derive: false,
        main: "fn main() { let mut customer = Customer { email: String::new(), credit_limit_cents: 10_000 }; update(&mut customer).expect(\"更新客户\"); assert_eq!(customer.email, \"ada@corp.example\"); assert_eq!(customer.credit_limit_cents, 75_000); }",
    },
    Snippet {
        file: "derive/README.md",
        heading: "### Read a reflected field",
        prefix_heading: None,
        derive: true,
        main: "fn main() { let user = User { name: String::from(\"Ada\") }; assert_eq!(read_name(&user).expect(\"read reflected name\"), \"Ada\"); }",
    },
    Snippet {
        file: "derive/README.zh_CN.md",
        heading: "### 读取反射字段",
        prefix_heading: None,
        derive: true,
        main: "fn main() { let user = User { name: String::from(\"Ada\") }; assert_eq!(read_name(&user).expect(\"读取反射字段\"), \"Ada\"); }",
    },
    Snippet {
        file: "doc/user_guide.md",
        heading: "## Read, update, and invoke",
        prefix_heading: Some("## Add reflection to a type"),
        derive: false,
        main: "fn main() { let mut customer = Customer { email: String::new(), credit_limit_cents: 10_000, internal_note: String::new() }; update(&mut customer).expect(\"update customer\"); assert_eq!(customer.email, \"ada@corp.example\"); assert_eq!(customer.credit_limit_cents, 75_000); }",
    },
    Snippet {
        file: "doc/user_guide.zh_CN.md",
        heading: "## 读取、更新和调用",
        prefix_heading: Some("## 在类型上启用反射"),
        derive: false,
        main: "fn main() { let mut customer = Customer { email: String::new(), credit_limit_cents: 10_000, internal_note: String::new() }; update(&mut customer).expect(\"更新客户\"); assert_eq!(customer.email, \"ada@corp.example\"); assert_eq!(customer.credit_limit_cents, 75_000); }",
    },
];

fn rust_block_after(markdown: &str, heading: &str) -> Option<String> {
    let level = heading.chars().take_while(|ch| *ch == '#').count();
    if !(1..=6).contains(&level) || markdown.lines().filter(|line| line.trim() == heading).count() != 1 {
        return None;
    }
    let section = markdown.split_once(heading)?.1;
    let section = section
        .split_inclusive('\n')
        .take_while(|line| {
            let trimmed = line.trim_start();
            let next_level = trimmed.chars().take_while(|ch| *ch == '#').count();
            !(next_level > 0 && next_level <= level && trimmed.as_bytes().get(next_level) == Some(&b' '))
        })
        .collect::<String>();
    if section.lines().filter(|line| line.trim() == "```rust").count() != 1 {
        return None;
    }
    let mut lines = section.lines();
    while let Some(line) = lines.next() {
        if line.trim() == "```rust" {
            let mut code = Vec::new();
            for line in lines.by_ref() {
                if line.trim() == "```" {
                    return Some(code.join("\n"));
                }
                code.push(line);
            }
            return None;
        }
        if line.starts_with("## ") {
            return None;
        }
    }
    None
}

fn toml_string(value: &str) -> String {
    let mut escaped = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            ch if ch.is_control() => escaped.push_str(&format!("\\u{:04X}", ch as u32)),
            ch => escaped.push(ch),
        }
    }
    escaped.push('"');
    escaped
}

fn unique_temp_dir() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is after Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("qubit-reflect-doc-snippet-{}-{nanos}", std::process::id()))
}

fn compile_and_run(root: &Path, temp_root: &Path, snippet: &Snippet) -> Result<(), String> {
    let markdown_path = root.join(snippet.file);
    let markdown = fs::read_to_string(&markdown_path)
        .map_err(|error| format!("{}: failed to read Markdown: {error}", snippet.file))?;
    let mut code = match snippet.prefix_heading {
        Some(heading) => rust_block_after(&markdown, heading)
            .ok_or_else(|| format!("{}: no Rust code block under {heading}", snippet.file))?,
        None => String::new(),
    };
    let body = rust_block_after(&markdown, snippet.heading)
        .ok_or_else(|| format!("{}: no Rust code block under {}", snippet.file, snippet.heading))?;
    code.push_str("\n\n");
    code.push_str(&body);
    let temp_dir = temp_root.join(snippet.file.replace(['/', '.'], "-"));
    let result = (|| {
        fs::create_dir_all(temp_dir.join("src"))
            .map_err(|error| format!("{}: create temp consumer: {error}", snippet.file))?;
        let runtime_path = toml_string(&root.display().to_string());
        let derive_dep = if snippet.derive {
            format!(
                "\nqubit-reflect-derive = {{ path = {} }}",
                toml_string(&root.join("derive").display().to_string())
            )
        } else {
            String::new()
        };
        let source = format!("{code}\n{}\n", snippet.main);
        fs::write(
            temp_dir.join("Cargo.toml"),
            format!(
                "[package]\nname = \"doc-snippet\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[workspace]\n\n[dependencies]\nqubit-reflect = {{ path = {runtime_path}, features = [\"derive\"] }}{derive_dep}\n"
            ),
        )
        .map_err(|error| format!("{}: write consumer manifest: {error}", snippet.file))?;
        fs::write(temp_dir.join("src/main.rs"), source)
            .map_err(|error| format!("{}: write consumer source: {error}", snippet.file))?;
        let output = Command::new("cargo")
            .arg("run")
            .arg("--offline")
            .arg("--manifest-path")
            .arg(temp_dir.join("Cargo.toml"))
            .env("CARGO_TARGET_DIR", temp_root.join("target"))
            .output()
            .map_err(|error| format!("{}: could not start cargo: {error}", snippet.file))?;
        if !output.status.success() {
            return Err(format!(
                "{} ({}): cargo run failed with {}\nstdout:\n{}\nstderr:\n{}",
                snippet.file,
                snippet.heading,
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        Ok(())
    })();
    let cleanup = fs::remove_dir_all(&temp_dir);
    match (result, cleanup) {
        (Err(error), _) => Err(error),
        (Ok(()), Err(error)) => Err(format!("{}: remove temp consumer: {error}", snippet.file)),
        (Ok(()), Ok(())) => Ok(()),
    }
}

#[test]
fn documentation_body_snippets_compile_and_run() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let temp_root = unique_temp_dir();
    fs::create_dir_all(&temp_root).expect("create isolated temporary consumers directory");
    let failures: Vec<_> = SNIPPETS
        .iter()
        .filter_map(|snippet| match compile_and_run(root, &temp_root, snippet) {
            Ok(()) => {
                eprintln!("PASS {} ({})", snippet.file, snippet.heading);
                None
            }
            Err(error) => Some(error),
        })
        .collect();
    fs::remove_dir_all(&temp_root).expect("remove isolated temporary consumers directory");
    assert!(
        failures.is_empty(),
        "documentation snippets failed:\n{}",
        failures.join("\n\n")
    );
}
