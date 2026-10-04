"""Focused tests for consumer isolation, profiles and archive validation."""
import importlib.util
import io
from pathlib import Path
import tarfile
import tempfile
import subprocess
import sys
import unittest
from unittest.mock import patch
import json
import tomllib

script_root = Path(__file__).resolve().parent
if script_root.name == 'tests':
    script_root = script_root.parent
spec = importlib.util.spec_from_file_location('release_consumer', script_root / 'check_release_consumer.py')
subject = importlib.util.module_from_spec(spec)
spec.loader.exec_module(subject)


class ConsumerTests(unittest.TestCase):
    def package(self, name='qubit-reflect', source='registry+https://github.com/rust-lang/crates.io-index', path='/tmp/example/Cargo.toml'):
        return {'name': name, 'version': '0.1.0', 'source': source, 'manifest_path': path}

    def validate_sources(self, metadata, allowed, version, consumer_manifest=Path("/tmp/consumer/Cargo.toml")):
        if "resolve" not in metadata:
            root = {"id": "path+file:///tmp/consumer#release-consumer@0.0.0", "name": "release-consumer", "version": "0.0.0", "source": None, "manifest_path": "/tmp/consumer/Cargo.toml"}
            metadata = {"packages": [root, *metadata["packages"]], "resolve": {"root": root["id"]}}
        subject.validate_sources(metadata, allowed, version, consumer_manifest)

    def test_same_named_local_transitive_dependency_is_not_the_consumer_root(self):
        root = {"id": "path+file:///tmp/consumer#release-consumer@0.0.0", "name": "release-consumer", "version": "0.0.0", "source": None, "manifest_path": "/tmp/consumer/Cargo.toml"}
        for source in [None, "git+https://example.com/rogue"]:
            for version in ["0.0.0", "0.9.0"]:
                transitive = {"id": f"path+file:///tmp/transitive#release-consumer@{version}", "name": "release-consumer", "version": version, "source": source, "manifest_path": "/tmp/transitive/Cargo.toml"}
                metadata = {"packages": [root, self.package(), transitive], "resolve": {"root": root["id"]}}
                with self.subTest(source=source, version=version), self.assertRaisesRegex(ValueError, "non-registry dependency: release-consumer"):
                    self.validate_sources(metadata, {}, "0.1.0")

    def test_root_manifest_and_resolve_id_must_match(self):
        root = {"id": "consumer-id", "name": "release-consumer", "version": "0.0.0", "source": None, "manifest_path": "/tmp/consumer/Cargo.toml"}
        for root_id, manifest in [("wrong-id", Path(root["manifest_path"])), (root["id"], Path("/tmp/other/Cargo.toml"))]:
            metadata = {"packages": [root, self.package()], "resolve": {"root": root_id}}
            with self.subTest(root_id=root_id, manifest=manifest), self.assertRaisesRegex(ValueError, "consumer root"):
                self.validate_sources(metadata, {}, "0.1.0", manifest)

    def test_same_named_registry_dependency_is_still_valid(self):
        transitive = self.package(name="release-consumer")
        transitive["version"] = "0.9.0"
        self.validate_sources({"packages": [self.package(), transitive]}, {}, "0.1.0")

    def test_registry_accepts_registry_dependency(self):
        self.validate_sources({'packages': [self.package()]}, {}, '0.1.0')

    def test_registry_rejects_local_runtime_and_git_dependency(self):
        for source in [None, 'git+https://example.com/repo']:
            with self.subTest(source=source), self.assertRaises(ValueError):
                self.validate_sources({'packages': [self.package(source=source)]}, {}, '0.1.0')

    def test_staged_rejects_unapproved_local_upstream(self):
        packages = [self.package(source=None), self.package(name='qubit-id', source=None)]
        with self.assertRaises(ValueError):
            self.validate_sources({'packages': packages}, {'qubit-reflect': Path('/tmp/example')}, '0.1.0')

    def test_staged_rejects_wrong_source_path(self):
        with self.assertRaises(ValueError):
            self.validate_sources({'packages': [self.package(source=None)]}, {'qubit-reflect': Path('/tmp/other')}, '0.1.0')

    def test_reflection_versions_must_match(self):
        package = self.package()
        package['version'] = '0.2.0'
        with self.assertRaises(ValueError):
            self.validate_sources({'packages': [package]}, {}, '0.1.0')

    def test_registry_manifests_have_no_patch_or_path(self):
        for profile in subject.PROFILES:
            manifest, source = subject.fixture('0.1.0', profile, {})
            self.assertNotIn('[patch.', manifest)
            self.assertNotIn('path =', manifest)
            self.assertIn('exercise();', source)

    def test_qubit_profile_uses_staged_runtime_dependency_versions(self):
        with tempfile.TemporaryDirectory() as directory:
            runtime = Path(directory)
            (runtime / "Cargo.toml").write_text(
                '[package]\nname="qubit-reflect"\nversion="0.2.0"\n'
                '[dependencies]\nqubit-id={version="0.7.9"}\n'
                'qubit-datatype={version="0.14.3"}\n', encoding="utf-8")
            manifest, _ = subject.fixture("0.2.0", "qubit-only", {"qubit-reflect": runtime})
            dependencies = tomllib.loads(manifest)["dependencies"]
            self.assertEqual(dependencies["qubit-id"]["version"], "0.7.9")
            self.assertEqual(dependencies["qubit-datatype"], "0.14.3")

    def test_rejects_duplicate_qubit_id_versions(self):
        first = self.package(name="qubit-id")
        second = self.package(name="qubit-id")
        first["version"], second["version"] = "0.6.0", "0.7.0"
        with self.assertRaisesRegex(ValueError, "multiple qubit-id versions"):
            self.validate_sources({"packages": [self.package(), first, second]}, {}, "0.1.0")

    def test_profile_exercises_requested_builtins(self):
        manifest, source = subject.fixture('0.1.0', 'public-all', {})
        self.assertIn('default-features = false', manifest)
        for token in ['BigDecimal', 'NaiveDate', 'Uuid', 'qubit_id::Id', 'qubit_datatype::DataType', 'Grace']:
            self.assertIn(token, source)

    def test_archive_rejects_traversal(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            archive = root / 'bad.crate'
            with tarfile.open(archive, 'w:gz') as packed:
                member = tarfile.TarInfo('qubit-reflect-0.1.0/../../escaped')
                member.size = 1
                packed.addfile(member, io.BytesIO(b'x'))
            with self.assertRaises(ValueError):
                subject.unpack(archive, root / 'output', 'qubit-reflect', '0.1.0')


    def test_cli_rejects_invalid_mode_archive_combinations(self):
        cases = [
            ['--mode', 'registry', '--runtime-archive', '/tmp/runtime.crate'],
            ['--mode', 'staged', '--runtime-archive', '/tmp/runtime.crate'],
            ['--mode', 'derive-registry', '--runtime-archive', '/tmp/runtime.crate', '--derive-archive', '/tmp/derive.crate'],
        ]
        for case in cases:
            with self.subTest(case=case):
                result = subprocess.run([sys.executable, str(script_root / 'check_release_consumer.py'),
                    *case, '--version', '0.1.0', '--evidence-dir', '/tmp/unused-release-evidence'],
                    capture_output=True, text=True)
                self.assertEqual(result.returncode, 2, result.stderr)

    def test_cli_rejects_existing_evidence_directory(self):
        with tempfile.TemporaryDirectory() as directory:
            result = subprocess.run([sys.executable, str(script_root / 'check_release_consumer.py'),
                '--mode', 'registry', '--version', '0.1.0', '--evidence-dir', directory],
                capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('FileExistsError', result.stderr)

    def test_unknown_registry_is_rejected(self):
        with self.assertRaises(ValueError):
            self.validate_sources({'packages': [self.package(source='registry+https://example.com/index')]}, {}, '0.1.0')


class PackagedExampleTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.allowed = {name: self.root / name for name in ["qubit-reflect", "qubit-reflect-derive"]}
        for root in self.allowed.values():
            root.mkdir()
        (self.allowed["qubit-reflect"] / "Cargo.toml").write_text(
            '[package]\nname="qubit-reflect"\nversion="0.1.0"\n'
            '[dependencies]\nqubit-id={version="0.7"}\n'
            'qubit-datatype={version="0.14.0"}\n', encoding="utf-8")
        examples = self.allowed["qubit-reflect"] / "examples"
        examples.mkdir()
        for name in ["field_patch", "customer_patch", "support_action"]:
            (examples / f"{name}.rs").write_text("fn main() {}")

    def test_packaged_examples_are_a_separate_staged_consumer(self):
        self.assertTrue(callable(getattr(subject, "check_packaged_examples", None)),
                        "staged validation must run an independent packaged examples consumer")

    def test_manifest_bins_point_to_unpacked_sources_only(self):
        manifest = tomllib.loads(subject.packaged_examples_fixture("0.1.0", self.allowed))
        self.assertEqual(manifest["dependencies"]["qubit-reflect"]["version"], "=0.1.0")
        for binary in manifest["bin"]:
            self.assertEqual(Path(binary["path"]).parent, self.allowed["qubit-reflect"] / "examples")
        self.assertEqual(set(manifest["patch"]["crates-io"]), set(self.allowed))

    def test_missing_archive_or_source_never_falls_back_to_checkout(self):
        for allowed in [{}, {"qubit-reflect": self.allowed["qubit-reflect"]}]:
            with self.subTest(allowed=allowed), self.assertRaises(ValueError):
                subject.packaged_examples_fixture("0.1.0", allowed)
        (self.allowed["qubit-reflect"] / "examples/support_action.rs").unlink()
        with self.assertRaisesRegex(ValueError, "support_action"):
            subject.packaged_examples_fixture("0.1.0", self.allowed)

    def metadata(self, consumer=None):
        packages = [{"name": name, "version": "0.1.0", "source": None,
                     "manifest_path": str(root / "Cargo.toml")} for name, root in self.allowed.items()]
        consumer = consumer or self.root / "packaged-examples"
        root_id = f"path+file://{consumer}#release-consumer@0.0.0"
        packages.append({"id": root_id, "name": "release-consumer", "version": "0.0.0", "source": None,
                         "manifest_path": str(consumer / "Cargo.toml")})
        return {"packages": packages, "resolve": {"root": root_id}}

    def test_all_three_mains_run_and_evidence_records_sources(self):
        calls = []
        def run(command, **kwargs):
            calls.append(command)
            return subprocess.CompletedProcess(command, 0, json.dumps(self.metadata(kwargs["cwd"])) if "metadata" in command else "", "")
        with patch.object(subject.subprocess, "run", side_effect=run):
            results = {"profiles": {}}
            subject.check_packaged_examples(self.root, "0.1.0", self.allowed, {}, results)
        self.assertEqual([command[-1] for command in calls if "--bin" in command], ["field_patch", "customer_patch", "support_action"])
        self.assertEqual(results["packaged_example_sources"]["field_patch"], str(self.allowed["qubit-reflect"] / "examples/field_patch.rs"))

    def test_packaged_document_source_drift_is_rejected_before_cargo(self):
        runtime = self.allowed["qubit-reflect"]
        (runtime / "examples/field_patch.rs").write_text("// reflect-example-start\nfn main() {}\n// reflect-example-end\n")
        (runtime / "README.md").write_text("<!-- reflect-source: examples/field_patch.rs -->\n```rust\nfn main() { panic!(\"drift\"); }\n```\n")
        with patch.object(subject.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, json.dumps(self.metadata()), "")) as run:
            with self.assertRaisesRegex(RuntimeError, "Markdown/source drift"):
                subject.check_packaged_examples(self.root, "0.1.0", self.allowed, {}, {"profiles": {}})
            run.assert_not_called()

    def test_archive_links_and_special_files_are_rejected(self):
        for kind in [tarfile.SYMTYPE, tarfile.LNKTYPE, tarfile.FIFOTYPE]:
            archive = self.root / "unsafe.crate"
            with tarfile.open(archive, "w:gz") as packed:
                member = tarfile.TarInfo("qubit-reflect-0.1.0/unsafe")
                member.type = kind
                member.linkname = "/tmp/escape"
                packed.addfile(member)
            with self.subTest(kind=kind), self.assertRaisesRegex(ValueError, "links or special files"):
                subject.unpack(archive, self.root / "unpacked", "qubit-reflect", "0.1.0")

    def test_main_adds_packaged_consumer_only_for_staged_mode(self):
        for mode in ["staged", "registry", "derive-registry"]:
            archives = []
            roots = {}
            if mode != "registry":
                archive = self.root / "runtime.crate"
                archive.write_bytes(b"fixture")
                archives += ["--runtime-archive", str(archive)]
                roots["qubit-reflect"] = self.allowed["qubit-reflect"]
            if mode == "staged":
                archive = self.root / "derive.crate"
                archive.write_bytes(b"fixture")
                archives += ["--derive-archive", str(archive)]
                roots["qubit-reflect-derive"] = self.allowed["qubit-reflect-derive"]
            args = ["check_release_consumer.py", "--mode", mode, "--version", "0.1.0", "--evidence-dir", str(self.root / mode), *archives]
            def run(command, **kwargs):
                actual = self.metadata(kwargs["cwd"])
                for package in actual["packages"]:
                    if package["name"] not in roots and package["name"] != "release-consumer":
                        package["source"] = "registry+https://github.com/rust-lang/crates.io-index"
                return subprocess.CompletedProcess(command, 0, json.dumps(actual) if "metadata" in command else "", "")
            with self.subTest(mode=mode), patch.object(sys, "argv", args), patch.object(subject, "unpack", side_effect=lambda archive, destination, name, version: roots[name]), patch.object(subject.subprocess, "run", side_effect=run), patch.object(subject, "check_packaged_examples") as check:
                self.assertEqual(subject.main(), 0)
                self.assertEqual(check.call_count, int(mode == "staged"))
                summary = json.loads((self.root / mode / "summary.json").read_text())
                self.assertEqual(set(summary["profiles"]), set(subject.PROFILES))

    def test_main_failure_and_metadata_violation_fail_validation(self):
        for failing_bin, invalid_metadata in [(name, False) for name in ["field_patch", "customer_patch", "support_action"]] + [("customer_patch", True)]:
            def run(command, **kwargs):
                metadata = self.metadata(kwargs["cwd"])
                if invalid_metadata:
                    metadata["packages"][0]["manifest_path"] = str(self.root / "checkout/Cargo.toml")
                code = 1 if "--bin" in command and command[-1] == failing_bin else 0
                return subprocess.CompletedProcess(command, code, json.dumps(metadata) if "metadata" in command else "", "assertion failed")
            with self.subTest(failing_bin=failing_bin, invalid_metadata=invalid_metadata), patch.object(subject.subprocess, "run", side_effect=run):
                with self.assertRaises((RuntimeError, ValueError)):
                    subject.check_packaged_examples(self.root / f"{failing_bin}-{invalid_metadata}", "0.1.0", self.allowed, {}, {"profiles": {}})


if __name__ == '__main__':
    unittest.main()
