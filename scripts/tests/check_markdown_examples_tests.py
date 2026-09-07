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
