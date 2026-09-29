"""Regression tests for Markdown parsing and executable example validation."""

import importlib.util
import json
import pathlib
import tempfile
import unittest
import sys

SCRIPT = pathlib.Path(__file__).resolve().parents[1] / "check_markdown_examples.py"
SPEC = importlib.util.spec_from_file_location("markdown_examples", SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)


class ParserTests(unittest.TestCase):
    def parse(self, source):
        with tempfile.TemporaryDirectory() as directory:
            document = pathlib.Path(directory) / "guide.md"
            document.write_text(source)
            return MODULE.parse_document(document)

    def test_modes_and_source_lines(self):
        blocks = self.parse("```rust\nfn main() {}\n```\n\nLibrary only.\n``` rust, no_run \npub fn demo() {}\n```\n\nInvalid type.\n```rust,compile_fail\nlet x: u8 = false;\n```\n")
        self.assertEqual([block.mode for block in blocks], ["run", "no_run", "compile_fail"])
        self.assertEqual([block.line for block in blocks], [1, 6, 11])

    def test_rejects_invalid_blocks_with_location(self):
        for source in ["```rust,ignore\nx\n```", "```rust\n```", "```rust\nfn main() {}", "plain", "```rust,no_run\npub fn x() {}\n```"]:
            with self.subTest(source=source), self.assertRaisesRegex(MODULE.ExampleError, r"guide.md:\d+"):
                self.parse(source)

    def test_does_not_parse_rust_fences_inside_other_blocks(self):
        blocks = self.parse("````text\n```rust\nnot rust\n```\n````\n```rust\nfn main() {}\n```\n")
        self.assertEqual(len(blocks), 1)
        self.assertEqual(blocks[0].line, 6)

    def test_lock_drift_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            baseline = pathlib.Path(directory) / "baseline.lock"
            resolved = pathlib.Path(directory) / "resolved.lock"
            baseline.write_text('version=4\n[[package]]\nname="shared"\nversion="1.0.0"\nsource="registry+test"\n')
            resolved.write_text(baseline.read_text().replace('1.0.0', '1.0.1'))
            with self.assertRaisesRegex(MODULE.ExampleError, "lock drift: shared"):
                MODULE.verify_lock(baseline, resolved)

    def test_real_inventory_examples_are_isolated(self):
        root = SCRIPT.parents[1]
        source = '''```rust
use qubit_reflect::{Reflect, ReflectRegistry};
#[derive(Reflect)]
#[reflect(rename = "IsolatedExample")]
struct Example;
fn main() {
    let registry = ReflectRegistry::initialize().unwrap();
    assert!(registry.get(Example::type_descriptor().type_id()).is_some());
}
```
'''
        with tempfile.TemporaryDirectory() as directory:
            document = pathlib.Path(directory) / "inventory.md"
            document.write_text(source + "\n" + source)
            self.assertEqual(MODULE.check_examples(root, [document]), 2)

    def test_derive_readmes_compile_with_direct_macro_dependency(self):
        root = SCRIPT.parents[1]
        documents = [root / "derive/README.md", root / "derive/README.zh_CN.md"]
        self.assertEqual(MODULE.check_examples(root, documents), 2)


class SourceReferenceTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = pathlib.Path(self.directory.name)
        (self.root / "examples").mkdir()
        self.source = self.root / "examples/demo.rs"
        self.source.write_text("// SPDX-License-Identifier: MIT\n// reflect-example-start\nfn main() {}\n// reflect-example-end\n")
        self.document = self.root / "guide.md"

    def parse(self, body="fn main() {}", reference="examples/demo.rs", mode="rust"):
        self.document.write_text(f"<!-- reflect-source: {reference} -->\n\n```{mode}\n{body}\n```\n")
        return MODULE.parse_document(self.document, self.root)

    def test_reference_drift_is_not_silently_accepted(self):
        self.assertEqual(self.parse()[0].source, "fn main() {}\n")
        self.document.write_text("<!-- reflect-source: examples/demo.rs -->\n```rust\nfn main() { panic!(\"drift\"); }\n```\n")
        with self.assertRaisesRegex(MODULE.ExampleError, "Markdown/source drift"):
            MODULE.parse_document(self.document, self.root)

    def test_license_separator_header_is_allowed(self):
        self.source.write_text("// ========\n//    Copyright (c) 2026 Example.\n//\n//    SPDX-License-Identifier: Apache-2.0\n//\n//    Licensed under the Apache License, Version 2.0.\n// ========\n\n// reflect-example-start\nfn main() {}\n// reflect-example-end\n")
        try:
            blocks = self.parse()
        except MODULE.ExampleError as error:
            self.fail(f"a valid license header must be accepted: {error}")
        self.assertEqual(blocks[0].source, "fn main() {}\n")

    def test_equal_region_and_reference(self):
        self.assertEqual(self.parse()[0].source_reference, "examples/demo.rs")

    def test_translated_code_drift_has_document_and_source_location(self):
        with self.assertRaisesRegex(MODULE.ExampleError, r"guide.md:3.*examples/demo.rs"):
            self.parse("fn main() { assert!(false); }")

    def test_whitespace_and_comments_are_not_stripped(self):
        for body in ["fn main() {} ", "// translated comment\nfn main() {}", "fn main() {}\n"]:
            with self.subTest(body=body), self.assertRaisesRegex(MODULE.ExampleError, "Markdown/source drift"):
                self.parse(body)

    def test_crlf_is_normalized(self):
        self.source.write_bytes(self.source.read_bytes().replace(b"\n", b"\r\n"))
        self.assertEqual(self.parse()[0].source, "fn main() {}\n")
        self.document.write_bytes(self.document.read_bytes().replace(b"\n", b"\r\n"))
        self.assertEqual(MODULE.parse_document(self.document, self.root)[0].source, "fn main() {}\n")

    def test_unicode_line_separators_are_content_not_normalized_newlines(self):
        for separator in ["\u2028", "\u2029", "\u0085", "\v", "\f", "\r"]:
            with self.subTest(separator=repr(separator)):
                program = f'fn main() {{ assert_eq!("a{separator}b".len(), {len(("a" + separator + "b").encode("utf-8"))}); }}'
                self.source.write_text("// reflect-example-start\n" + program + "\n// reflect-example-end\n")
                # A real, identical source/Markdown pair is accepted first.
                self.assertEqual(self.parse(program)[0].source_reference, "examples/demo.rs")
                with self.assertRaisesRegex(MODULE.ExampleError, r"guide.md:3.*examples/demo.rs.*Markdown/source drift"):
                    self.parse(program.replace(separator, "\n"))
                self.assertEqual(self.parse(program)[0].source, program + "\n")

    def test_missing_duplicate_unclosed_regions_and_hidden_code(self):
        for content in ["fn main() {}", "// reflect-example-start\nfn main() {}", self.source.read_text() * 2,
                        "fn hidden() {}\n" + self.source.read_text()]:
            with self.subTest(content=content):
                self.source.write_text(content)
                with self.assertRaisesRegex(MODULE.ExampleError, r"guide.md:3.*examples/demo.rs"):
                    self.parse()

    def test_paths_reject_absolute_parent_and_symlink_escape(self):
        outside = self.root.parent / (self.root.name + "-outside.rs")
        # A sibling target need not exist: resolving the link must still reject escape.
        (self.root / "examples/link.rs").symlink_to(outside)
        for reference in [str(self.source), "examples/../examples/demo.rs", "examples/link.rs", "examples/missing.rs"]:
            with self.subTest(reference=reference), self.assertRaisesRegex(MODULE.ExampleError, r"guide.md:3"):
                self.parse(reference=reference)

    def test_reference_requires_adjacent_run_fence(self):
        for body in ["text\n```rust\nfn main() {}\n```", "```rust,no_run\nfn main() {}\n```",
                     "```rust,compile_fail\nfn main() {}\n```", "```text\nx\n```", "```rust\nfn main() {}", "```rust\n```", ""]:
            self.document.write_text("<!-- reflect-source: examples/demo.rs -->\n" + body)
            with self.subTest(body=body), self.assertRaisesRegex(MODULE.ExampleError, r"guide.md:\d+.*examples/demo.rs"):
                MODULE.parse_document(self.document, self.root)


class CargoTests(unittest.TestCase):
    """Real Cargo executions: compiler success alone is insufficient."""

    def check(self, source, timeout=10):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            (root / "Cargo.toml").write_text('[package]\nname="fixture-dependency"\nversion="0.0.0"\nedition="2024"\n')
            (root / "src").mkdir()
            (root / "src/lib.rs").write_text("")
            (root / "Cargo.lock").write_text('version = 4\n[[package]]\nname="fixture-dependency"\nversion="0.0.0"\n')
            document = root / "guide.md"
            document.write_text(source)
            return MODULE.check_examples(root, [document], timeout=timeout, dependency_name="fixture-dependency")

    def test_success_result_main_and_compile_modes(self):
        self.check('```rust\nfn main() -> Result<(), String> { assert_eq!(2 + 2, 4); Ok(()) }\n```\n\nA library API.\n```rust,no_run\npub fn value() -> u8 { 1 }\n```\n\nWrong return type.\n```rust,compile_fail\npub fn value() -> u8 { false }\n```\n')

    def test_runtime_assertion_failure_is_rejected(self):
        with self.assertRaisesRegex(MODULE.ExampleError, r"guide.md:1.*run"):
            self.check('```rust\nfn main() { assert_eq!(1, 2); }\n```')

    def test_timeout_is_rejected(self):
        with self.assertRaisesRegex(MODULE.ExampleError, "timeout") as caught:
            self.check('```rust\nfn main() { std::thread::sleep(std::time::Duration::from_secs(10)); }\n```', timeout=0.1)
        evidence = pathlib.Path(str(caught.exception).split("Evidence retained at ")[-1])
        commands = [json.loads(line) for line in (evidence / "commands.jsonl").read_text().splitlines()]
        run = next(command for command in commands if command["timeout"])
        self.assertIsNotNone(run["exit_code"])
        if pathlib.Path("/proc").is_dir():
            self.assertFalse(pathlib.Path(f"/proc/{run['pid']}").exists(), "timed-out child must be reaped")

    def test_compile_fail_must_fail_compilation(self):
        with self.assertRaisesRegex(MODULE.ExampleError, "compiled successfully"):
            self.check('Expected a type error.\n```rust,compile_fail\npub fn valid() {}\n```')

    def test_build_failure_is_rejected(self):
        with self.assertRaisesRegex(MODULE.ExampleError, "compile"):
            self.check('```rust\nfn main() { missing_function(); }\n```')


if __name__ == "__main__":
    unittest.main()
