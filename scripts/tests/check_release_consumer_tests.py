"""Focused tests for consumer isolation, profiles and archive validation."""
import importlib.util
import io
from pathlib import Path
import tarfile
import tempfile
import subprocess
import sys
import unittest

script_root = Path(__file__).resolve().parent
if script_root.name == 'tests':
    script_root = script_root.parent
spec = importlib.util.spec_from_file_location('release_consumer', script_root / 'check_release_consumer.py')
subject = importlib.util.module_from_spec(spec)
spec.loader.exec_module(subject)


class ConsumerTests(unittest.TestCase):
    def package(self, name='qubit-reflect', source='registry+https://github.com/rust-lang/crates.io-index', path='/tmp/example/Cargo.toml'):
        return {'name': name, 'version': '0.1.0', 'source': source, 'manifest_path': path}

    def test_registry_accepts_registry_dependency(self):
        subject.validate_sources({'packages': [self.package()]}, {}, '0.1.0')

    def test_registry_rejects_local_runtime_and_git_dependency(self):
        for source in [None, 'git+https://example.com/repo']:
            with self.subTest(source=source), self.assertRaises(ValueError):
                subject.validate_sources({'packages': [self.package(source=source)]}, {}, '0.1.0')

    def test_staged_rejects_unapproved_local_upstream(self):
        packages = [self.package(source=None), self.package(name='qubit-id', source=None)]
        with self.assertRaises(ValueError):
            subject.validate_sources({'packages': packages}, {'qubit-reflect': Path('/tmp/example')}, '0.1.0')

    def test_staged_rejects_wrong_source_path(self):
        with self.assertRaises(ValueError):
            subject.validate_sources({'packages': [self.package(source=None)]}, {'qubit-reflect': Path('/tmp/other')}, '0.1.0')

    def test_reflection_versions_must_match(self):
        package = self.package()
        package['version'] = '0.2.0'
        with self.assertRaises(ValueError):
            subject.validate_sources({'packages': [package]}, {}, '0.1.0')

    def test_registry_manifests_have_no_patch_or_path(self):
        for profile in subject.PROFILES:
            manifest, source = subject.fixture('0.1.0', profile, {})
            self.assertNotIn('[patch.', manifest)
            self.assertNotIn('path =', manifest)
            self.assertIn('exercise();', source)

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
            subject.validate_sources({'packages': [self.package(source='registry+https://example.com/index')]}, {}, '0.1.0')


if __name__ == '__main__':
    unittest.main()
