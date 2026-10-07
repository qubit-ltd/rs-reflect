"""Document facts are checked against bilingual contracts and actual inputs."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "check_document_contracts.py"
SPEC = importlib.util.spec_from_file_location("document_contracts", SCRIPT)
subject = importlib.util.module_from_spec(SPEC) if SCRIPT.exists() else None
if subject:
    SPEC.loader.exec_module(subject)

GROUPS = {
    "README": ("README.md", "README.zh_CN.md", ["facade.explicit", "examples.native"]),
    "derive": ("derive/README.md", "derive/README.zh_CN.md", ["facade.explicit", "provider.qualified"]),
    "guide": ("doc/user_guide.md", "doc/user_guide.zh_CN.md", ["facade.explicit", "provider.qualified", "panic.async_poll", "panic.abort", "dispatch.input_recovery", "examples.native"]),
    "design": (
        "doc/2026-09-03-qubit-reflect-design.md",
        "doc/2026-09-03-qubit-reflect-design.zh_CN.md",
        [
            "panic.async_poll",
            "panic.abort",
            "dispatch.input_recovery",
            "coverage.configure",
            "coverage.parse",
            "coverage.validate",
            "coverage.expand",
            "registry.source",
        ],
    ),
    "stability": ("doc/2026-09-07-qubit-reflect-api-stability.md", "doc/2026-09-07-qubit-reflect-api-stability.zh_CN.md", ["dispatch.input_recovery"]),
    "matrix": ("doc/derive-contract-matrix.md", "doc/derive-contract-matrix.zh_CN.md", ["coverage.configure", "coverage.parse", "coverage.validate", "coverage.expand"]),
}
FACTS = {"facade.explicit": "qubit_reflect", "provider.qualified": "custom-provider", "panic.async_poll": "not-caught", "panic.abort": "catching-unavailable", "dispatch.input_recovery": "original-input", "examples.native": "cargo-example", "coverage.configure": "70", "coverage.parse": "85", "coverage.validate": "80", "coverage.expand": "85", "registry.source": "src/registry/reflect_registry.rs"}
class ContractTests(unittest.TestCase):
    def setUp(self):
        self.assertIsNotNone(subject, "document contract checker has not been implemented")
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        for english, chinese, keys in GROUPS.values():
            for name in [english, chinese]:
                path = self.root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                values = FACTS
                markers = "".join(
                    f"<!-- reflect-contract: {key}={values[key]} -->\n" for key in keys
                )
                path.write_text(markers + "Run `cargo metadata --locked --format-version 1`.\n")
        coverage = self.root / "scripts/derive_coverage_report.py"
        coverage.parent.mkdir()
        coverage.write_text("MINIMUM_LINE_COVERAGE = {'configure': 70.0, 'parse': 85.0, 'validate': 80.0, 'expand': 85.0}\n")
        registry = self.root / FACTS["registry.source"]
        registry.parent.mkdir(parents=True)
        registry.write_text("// registry")
        (self.root / "Cargo.toml").write_text('[package]\nname="fixture"\nversion="0.1.0"\ninclude=["/examples/**"]\n' + "".join(f'[[example]]\nname="{name}"\npath="examples/{name}.rs"\nrequired-features=["derive"]\n' for name in ["field_patch", "customer_patch", "support_action"]))
        (self.root / "examples").mkdir()
        for name in ["field_patch", "customer_patch", "support_action"]:
            (self.root / f"examples/{name}.rs").write_text("fn main() {}")

    def test_valid_bilingual_facts_and_real_inputs(self):
        subject.check_contracts(self.root)

    def test_duplicate_missing_unknown_and_bilingual_value_drift(self):
        document = self.root / "README.zh_CN.md"
        original = document.read_text()
        cases = [original + "<!-- reflect-contract: facade.explicit=qubit_reflect -->", original.splitlines()[0], original + "<!-- reflect-contract: extra.key=value -->", original.replace("qubit_reflect", "wrong")]
        for text in cases:
            with self.subTest(text=text):
                document.write_text(text)
                with self.assertRaisesRegex(ValueError, r"README.zh_CN.md"):
                    subject.check_contracts(self.root)

    def test_actual_coverage_threshold_change_is_detected(self):
        source = self.root / "scripts/derive_coverage_report.py"
        source.write_text(source.read_text().replace("70.0", "71.0"))
        with self.assertRaisesRegex(ValueError, "coverage.configure"):
            subject.check_contracts(self.root)

    def test_actual_example_configuration_and_registry_path_are_required(self):
        manifest = self.root / "Cargo.toml"
        original = manifest.read_text()
        for changed in [original.replace('required-features=["derive"]', 'required-features=[]', 1), original.replace('/examples/**', '/src/**'), original.replace('[package]\n', '[package]\nautoexamples=false\n')]:
            with self.subTest(changed=changed):
                manifest.write_text(changed)
                with self.assertRaises(ValueError):
                    subject.check_contracts(self.root)
        manifest.write_text(original)
        (self.root / FACTS["registry.source"]).unlink()
        with self.assertRaisesRegex(ValueError, "registry.source"):
            subject.check_contracts(self.root)

    def test_source_checkout_docs_describe_standalone_registry_resolution(self):
        for name in (
            "README.md", "README.zh_CN.md", "doc/user_guide.md", "doc/user_guide.zh_CN.md",
            "derive/README.md", "derive/README.zh_CN.md",
        ):
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("Run `cargo metadata --locked --format-version 1` from this checkout.\n")
        subject.check_source_checkout_docs(self.root)
        document = self.root / "doc/user_guide.md"
        document.write_text("Run `./.infra/bin/prepare-local-path-dependencies.sh` first.\n")
        with self.assertRaisesRegex(ValueError, "prepare-local-path-dependencies"):
            subject.check_source_checkout_docs(self.root)
        document.write_text(
            "Keep the sibling rust-common/rs-id and rust-common/rs-datatype checkouts.\n"
            "Run `cargo metadata --locked --format-version 1`.\n"
        )
        with self.assertRaisesRegex(ValueError, "registry dependencies"):
            subject.check_source_checkout_docs(self.root)

    def test_derive_readmes_reject_obsolete_sibling_dependency_guidance(self):
        cases = {
            "derive/README.md": "Keep the sibling rust-common/rs-id and rust-common/rs-datatype checkouts.",
            "derive/README.zh_CN.md": "请检出相邻 rust-common/rs-id 与 rust-common/rs-datatype 仓库。",
        }
        for name, obsolete_guidance in cases.items():
            for document in (
                "README.md", "README.zh_CN.md", "doc/user_guide.md", "doc/user_guide.zh_CN.md",
                "derive/README.md", "derive/README.zh_CN.md",
            ):
                valid = self.root / document
                valid.parent.mkdir(parents=True, exist_ok=True)
                valid.write_text("Run `cargo metadata --locked --format-version 1`.\n")
            path = self.root / name
            path.write_text(
                obsolete_guidance + "\nRun `cargo metadata --locked --format-version 1`.\n"
            )
            with self.subTest(document=name), self.assertRaisesRegex(ValueError, "registry dependencies"):
                subject.check_source_checkout_docs(self.root)

if __name__ == "__main__":
    unittest.main()
