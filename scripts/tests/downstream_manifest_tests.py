#!/usr/bin/env python3

import contextlib
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest import mock


SCRIPT = Path(__file__).resolve().parents[1] / "downstream_manifest.py"
WORKFLOW = Path(__file__).resolve().parents[2] / ".github/workflows/ci.yml"
SPEC = importlib.util.spec_from_file_location("downstream_manifest", SCRIPT)
downstream_manifest = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(downstream_manifest)


REPOSITORIES = [
    ("qubit-ltd/rs-model-metadata", "rust-platform/rs-model-metadata"),
    ("qubit-ltd/rs-platform", "rust-platform/rs-platform"),
    ("qubit-ltd/rs-id", "rust-common/rs-id"),
    ("qubit-ltd/rs-datatype", "rust-common/rs-datatype"),
    ("qubit-ltd/rs-redact", "rust-common/rs-redact"),
    ("qubit-ltd/rs-validator", "rust-common/rs-validator"),
    ("qubit-ltd/rs-validation-rules", "rust-common/rs-validation-rules"),
]
REVISIONS = [character * 40 for character in "1234567"]


class DownstreamManifestTests(unittest.TestCase):
    def setUp(self):
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
        self.manifest_path = self.root / "manifest.json"
        self.output_path = self.root / "collected.json"

    def tearDown(self):
        self.temporary_directory.cleanup()

    def manifest(self):
        return {
            "schema_version": 1,
            "repositories": [
                {"repository": repository, "path": path, "revision": revision}
                for (repository, path), revision in zip(REPOSITORIES, REVISIONS)
            ],
        }

    def write_manifest(self, manifest=None):
        self.manifest_path.write_text(
            json.dumps(self.manifest() if manifest is None else manifest),
            encoding="utf-8",
        )

    def invoke(self, *arguments):
        stdout = io.StringIO()
        stderr = io.StringIO()
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            return_code = downstream_manifest.main(list(arguments))
        return return_code, stdout.getvalue(), stderr.getvalue()

    def assert_invalid(self, manifest, diagnostic):
        self.write_manifest(manifest)
        return_code, _, stderr = self.invoke(
            "validate", "--manifest", str(self.manifest_path)
        )
        self.assertEqual(return_code, 1)
        self.assertIn(diagnostic, stderr)

    def test_validate_accepts_exact_manifest(self):
        self.write_manifest()
        return_code, _, stderr = self.invoke(
            "validate", "--manifest", str(self.manifest_path)
        )
        self.assertEqual(return_code, 0, stderr)

    def test_validate_accepts_a_fixed_reflect_baseline_revision(self):
        manifest = self.manifest()
        manifest["reflect_baseline_revision"] = "a" * 40
        self.write_manifest(manifest)
        return_code, _, stderr = self.invoke(
            "validate", "--manifest", str(self.manifest_path)
        )
        self.assertEqual(return_code, 0, stderr)

    def test_validate_rejects_an_invalid_reflect_baseline_revision(self):
        manifest = self.manifest()
        manifest["reflect_baseline_revision"] = "main"
        self.assert_invalid(manifest, "reflect_baseline_revision")

    def test_validate_rejects_missing_repository(self):
        manifest = self.manifest()
        manifest["repositories"].pop()
        self.assert_invalid(manifest, "missing repository")

    def test_validate_rejects_duplicate_repository(self):
        manifest = self.manifest()
        manifest["repositories"][-1] = dict(manifest["repositories"][0])
        self.assert_invalid(manifest, "duplicate repository")

    def test_validate_rejects_short_sha(self):
        manifest = self.manifest()
        manifest["repositories"][0]["revision"] = "a" * 39
        self.assert_invalid(manifest, "revision")

    def test_validate_rejects_main_revision(self):
        manifest = self.manifest()
        manifest["repositories"][0]["revision"] = "main"
        self.assert_invalid(manifest, "revision")

    def test_validate_rejects_absolute_path(self):
        manifest = self.manifest()
        manifest["repositories"][0]["path"] = "/tmp/rs-model-metadata"
        self.assert_invalid(manifest, "path")

    def test_validate_rejects_parent_path(self):
        manifest = self.manifest()
        manifest["repositories"][0]["path"] = "rust-platform/../rs-model-metadata"
        self.assert_invalid(manifest, "path")

    def test_validate_rejects_valid_relative_path_for_wrong_repository(self):
        manifest = self.manifest()
        manifest["repositories"][0]["path"] = "rust-platform/rs-platform"
        self.assert_invalid(manifest, "path does not match repository")

    def test_validate_rejects_unknown_repository(self):
        manifest = self.manifest()
        manifest["repositories"][0]["repository"] = "qubit-ltd/unknown"
        self.assert_invalid(manifest, "repository")

    def test_validate_rejects_unknown_fields(self):
        manifest = self.manifest()
        manifest["extra"] = True
        self.assert_invalid(manifest, "unknown field")

    def test_validate_rejects_unknown_schema_version(self):
        manifest = self.manifest()
        manifest["schema_version"] = 2
        self.assert_invalid(manifest, "schema_version")

    def test_matrix_emits_fixed_revisions_for_baseline(self):
        self.write_manifest()
        return_code, stdout, stderr = self.invoke(
            "matrix", "--manifest", str(self.manifest_path), "--mode", "baseline"
        )
        self.assertEqual(return_code, 0, stderr)
        self.assertEqual(
            json.loads(stdout),
            {
                "repositories": {
                    repository.rsplit("/", 1)[-1].replace("-", "_"): {
                        "repository": repository,
                        "path": path,
                        "revision": revision,
                    }
                    for (repository, path), revision in zip(REPOSITORIES, REVISIONS)
                }
            },
        )

    def test_matrix_emits_main_for_head(self):
        return_code, stdout, stderr = self.invoke(
            "matrix", "--manifest", str(self.manifest_path), "--mode", "head"
        )
        self.assertEqual(return_code, 0, stderr)
        self.assertEqual(
            {entry["revision"] for entry in json.loads(stdout)["repositories"].values()},
            {"main"},
        )

    def test_workflow_caches_are_channel_and_input_scoped(self):
        workflow = WORKFLOW.read_text(encoding="utf-8")
        for channel in ("baseline", "head"):
            start = workflow.index(f"  downstream-{channel}:")
            end = workflow.find("\n  downstream-", start + 1)
            job = workflow[start:] if end == -1 else workflow[start:end]
            self.assertIn(f"name: Cache {channel} Cargo state", job)
            self.assertIn(f"key: downstream-{channel}-", job)
            self.assertIn("toolchain-1.94.0-manifest-${{ hashFiles(", job)
            self.assertIn("-locks-${{ hashFiles(", job)

    def test_workflow_runs_real_baseline_and_head_gates(self):
        workflow = WORKFLOW.read_text(encoding="utf-8")
        for channel in ("baseline", "head"):
            with self.subTest(channel=channel):
                start = workflow.index(f"  downstream-{channel}:")
                if channel == "baseline":
                    end = workflow.index("\n  downstream-head:", start + 1)
                    job = workflow[start:end]
                else:
                    job = workflow[start:]
                self.assertIn("scripts/check-downstream.sh", job)
                self.assertIn("scripts/check_dependency_layout.py", job)
                self.assertIn("secrets.DEPENDENCY_TOKEN", job)
                self.assertIn("Require private dependency access", job)
                self.assertNotIn("continue-on-error", job)
                for repository in REPOSITORIES:
                    self.assertIn(repository[1], job)
        head = workflow[workflow.index("  downstream-head:"):]
        self.assertIn('gh api "repos/$repository/commits/main"', head)
        self.assertIn("downstream_manifest.py collect", head)

    def test_baseline_checks_the_pinned_reflect_version_at_its_cargo_path(self):
        workflow = WORKFLOW.read_text(encoding="utf-8")
        baseline = workflow[
            workflow.index("  downstream-baseline:"):workflow.index("\n  downstream-head:")
        ]
        self.assertIn('path: rust-platform/rs-reflect-ci', baseline)
        self.assertIn("reflect_baseline_revision", baseline)
        self.assertIn("path: rust-platform/rs-reflect,", baseline)
        self.assertIn("working-directory: rust-platform/rs-reflect-ci", baseline)

    def test_downstream_head_runs_staged_release_consumer(self):
        workflow = WORKFLOW.read_text(encoding="utf-8")
        head = workflow.split("\n  downstream-head:\n", 1)[1]
        for required in (
            "package -p qubit-reflect-derive",
            "package -p qubit-reflect",
            "--mode staged",
            "--runtime-archive",
            "--derive-archive",
            "release-consumer-evidence",
        ):
            self.assertIn(required, head)

    def test_head_resolve_validates_manifest_before_matrix(self):
        workflow = WORKFLOW.read_text(encoding="utf-8")
        start = workflow.index("  downstream-head:")
        job = workflow[start:]
        validation = job.index("downstream_manifest.py validate")
        matrix = job.index("downstream_manifest.py matrix")
        self.assertLess(validation, matrix)

    def test_derive_coverage_is_enabled_in_reusable_ci_job(self):
        workflow = WORKFLOW.read_text(encoding="utf-8")
        start = workflow.index("  rust-ci:")
        end = workflow.index("\n  downstream-baseline:", start)
        job = workflow[start:end]
        self.assertIn("uses: qubit-ltd/rs-infra-ci/.github/workflows/github-ci.yml@", job)
        self.assertIn("derive-coverage: true", job)

    def test_collect_rejects_dirty_checkout_without_writing_output(self):
        clean_heads = iter(REVISIONS)

        def run(argv, **kwargs):
            self.assertIsInstance(argv, list)
            if argv[-2:] == ["status", "--porcelain"]:
                path = Path(argv[2])
                output = " M Cargo.toml\n" if path.name == "rs-redact" else ""
            else:
                output = next(clean_heads) + "\n"
            return subprocess.CompletedProcess(argv, 0, output, "")

        with mock.patch.object(downstream_manifest.subprocess, "run", side_effect=run):
            return_code, _, stderr = self.invoke(
                "collect",
                "--layout-root",
                str(self.root),
                "--output",
                str(self.output_path),
            )
        self.assertEqual(return_code, 1)
        self.assertIn("dirty", stderr)
        self.assertFalse(self.output_path.exists())

    def test_collect_rejects_git_failure_without_writing_output(self):
        def run(argv, **kwargs):
            return subprocess.CompletedProcess(argv, 128, "", "not a repository")

        with mock.patch.object(downstream_manifest.subprocess, "run", side_effect=run):
            return_code, _, stderr = self.invoke(
                "collect",
                "--layout-root",
                str(self.root),
                "--output",
                str(self.output_path),
            )
        self.assertEqual(return_code, 1)
        self.assertIn("git rev-parse", stderr)
        self.assertFalse(self.output_path.exists())

    def test_collect_rejects_checkout_symlink_outside_layout(self):
        platform = self.root / "rust-platform"
        platform.mkdir()
        (platform / "rs-model-metadata").symlink_to(self.root.parent)
        with mock.patch.object(downstream_manifest.subprocess, "run") as run:
            return_code, _, stderr = self.invoke(
                "collect",
                "--layout-root",
                str(self.root),
                "--output",
                str(self.output_path),
            )
        self.assertEqual(return_code, 1)
        self.assertIn("escapes layout root", stderr)
        run.assert_not_called()

    def test_collect_writes_only_validated_heads(self):
        heads = iter(REVISIONS)

        def run(argv, **kwargs):
            self.assertIsInstance(argv, list)
            output = "" if argv[-2:] == ["status", "--porcelain"] else next(heads) + "\n"
            return subprocess.CompletedProcess(argv, 0, output, "")

        with mock.patch.object(downstream_manifest.subprocess, "run", side_effect=run):
            return_code, _, stderr = self.invoke(
                "collect",
                "--layout-root",
                str(self.root),
                "--output",
                str(self.output_path),
            )
        self.assertEqual(return_code, 0, stderr)
        self.assertEqual(json.loads(self.output_path.read_text()), self.manifest())

    def test_verify_rejects_head_mismatch(self):
        self.write_manifest()
        heads = iter(["f" * 40, *REVISIONS[1:]])

        def run(argv, **kwargs):
            output = "" if argv[-2:] == ["status", "--porcelain"] else next(heads) + "\n"
            return subprocess.CompletedProcess(argv, 0, output, "")

        with mock.patch.object(downstream_manifest.subprocess, "run", side_effect=run):
            return_code, _, stderr = self.invoke(
                "verify",
                "--layout-root",
                str(self.root),
                "--manifest",
                str(self.manifest_path),
            )
        self.assertEqual(return_code, 1)
        self.assertIn("HEAD mismatch", stderr)

    def test_verify_rejects_dirty_checkout(self):
        self.write_manifest()
        heads = iter(REVISIONS)

        def run(argv, **kwargs):
            if argv[-2:] == ["status", "--porcelain"]:
                output = "?? generated.txt\n" if Path(argv[2]).name == "rs-platform" else ""
            else:
                output = next(heads) + "\n"
            return subprocess.CompletedProcess(argv, 0, output, "")

        with mock.patch.object(downstream_manifest.subprocess, "run", side_effect=run):
            return_code, _, stderr = self.invoke(
                "verify",
                "--layout-root",
                str(self.root),
                "--manifest",
                str(self.manifest_path),
            )
        self.assertEqual(return_code, 1)
        self.assertIn("dirty", stderr)


if __name__ == "__main__":
    unittest.main()
