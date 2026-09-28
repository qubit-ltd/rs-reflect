"""Tests for the real downstream benchmark measurement tool."""

from __future__ import annotations

import importlib.util
from contextlib import ExitStack
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock


SCRIPT = Path(__file__).parents[1] / "measure-reflect-downstream.py"
SPEC = importlib.util.spec_from_file_location("measure_reflect_downstream", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
measure = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = measure
SPEC.loader.exec_module(measure)


def output(model_count: int = 133, ns: int = 100, iterations: int = 1) -> str:
    lines = ["platform/benchmark_version=3", f"platform/models={model_count}"]
    for name in measure.METRICS:
        lines.append(
            f"{name}: iterations={iterations}, ns/op={ns}, "
            f"allocations/op=10, allocated_bytes/op=100"
        )
    return "\n".join(lines)


def record(index: int, model_count: int = 133, ns: int = 100, iterations: int = 1) -> dict[str, object]:
    count, metrics = measure.parse_benchmark_output(output(model_count, ns, iterations))
    return {
        "sample": index,
        "command": ["cargo", "bench"],
        "exit_code": 0,
        "stdout": output(model_count, ns, iterations),
        "stderr": "",
        "model_count": count,
        "metrics": metrics,
    }


class MeasureReflectDownstreamTests(unittest.TestCase):
    def run_main_with_records(self, root: Path, records: list[dict[str, object]]) -> tuple[int, Path]:
        workspace = root / "superpowers-collect-test"
        workspace.mkdir()
        (workspace / ".superpowers-session").touch()
        platform_root = workspace / "inputs" / "rust-platform" / "rs-platform"
        platform_root.mkdir(parents=True)
        (platform_root / "Cargo.toml").write_text("[package]\n", encoding="utf-8")
        (platform_root.parent / "rs-model-metadata").mkdir()
        output_dir = workspace / "measurements"
        reflect_copy = workspace / "source-layout" / "rust-platform" / "rs-reflect"
        model_copy = workspace / "source-layout" / "rust-platform" / "rs-model-metadata"
        platform_copy = workspace / "source-layout" / "rust-platform" / "rs-platform"
        patches = (
            mock.patch.object(measure, "repository_facts", return_value={"rs-reflect": {"head": "a" * 40}}),
            mock.patch.object(measure, "lockfile_facts", return_value={"rs-platform": "b" * 64}),
            mock.patch.object(measure, "prepare_source_layout", return_value=(reflect_copy, model_copy, platform_copy)),
            mock.patch.object(measure, "_run_once", side_effect=records),
            mock.patch.object(measure.subprocess, "run", return_value=mock.Mock(stdout="rustc test\n")),
        )
        with ExitStack() as stack:
            for patcher in patches:
                stack.enter_context(patcher)
            result = measure.main([
                "--platform-root", str(platform_root),
                "--output-dir", str(output_dir),
                "--samples", "10",
            ])
        return result, output_dir / "measurements.json"

    def test_main_marks_a_complete_ten_sample_collection(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            result, report_path = self.run_main_with_records(
                Path(temporary), [record(index) for index in range(1, 11)]
            )

            payload = json.loads(report_path.read_text(encoding="utf-8"))
            self.assertEqual(result, 0)
            self.assertEqual(payload["schema_version"], 2)
            self.assertEqual(payload["status"], "complete")
            self.assertEqual(payload["expected_sample_count"], 10)
            self.assertEqual(payload["completed_sample_count"], 10)
            self.assertEqual(payload["successful_sample_count"], 10)
            self.assertIsNotNone(payload["summary"])
            self.assertIsNone(payload["error"])
            self.assertNotIn("summary_error", payload)
            self.assertNotIn("sample_count", payload)

    def test_main_preserves_model_drift_and_exits_nonzero(self) -> None:
        records = [record(index, 134 if index == 10 else 133) for index in range(1, 11)]
        with tempfile.TemporaryDirectory() as temporary:
            result, report_path = self.run_main_with_records(Path(temporary), records)

            payload = json.loads(report_path.read_text(encoding="utf-8"))
            self.assertEqual(result, 1)
            self.assertEqual(payload["status"], "failed")
            self.assertEqual(payload["completed_sample_count"], 10)
            self.assertEqual(payload["successful_sample_count"], 10)
            self.assertIsNone(payload["summary"])
            self.assertEqual(payload["error"]["kind"], "inconsistent_samples")
            self.assertEqual(payload["error"]["phase"], "aggregate")

    def test_source_tree_fingerprint_ignores_build_state_and_detects_input_changes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            source = Path(temporary) / "repo"
            source.mkdir()
            tracked = source / "Cargo.toml"
            tracked.write_text("[package]\n", encoding="utf-8")
            (source / "target").mkdir()
            (source / "target" / "artifact").write_text("one", encoding="utf-8")
            (source / ".git").write_text("worktree metadata", encoding="utf-8")

            initial = measure._source_tree_sha256(source)
            (source / "target" / "artifact").write_text("two", encoding="utf-8")
            self.assertEqual(measure._source_tree_sha256(source), initial)
            tracked.write_text("[package]\n# changed\n", encoding="utf-8")
            self.assertNotEqual(measure._source_tree_sha256(source), initial)

    def test_main_preserves_failed_sample_stderr_and_exits_nonzero(self) -> None:
        records = [record(index) for index in range(1, 10)]
        records.append({"sample": 10, "command": ["cargo", "bench"], "exit_code": 9, "stdout": "", "stderr": "compiler failed"})
        with tempfile.TemporaryDirectory() as temporary:
            result, report_path = self.run_main_with_records(Path(temporary), records)

            payload = json.loads(report_path.read_text(encoding="utf-8"))
            self.assertEqual(result, 1)
            self.assertEqual(payload["status"], "failed")
            self.assertEqual(payload["completed_sample_count"], 10)
            self.assertEqual(payload["successful_sample_count"], 9)
            self.assertIsNone(payload["summary"])
            self.assertEqual(payload["samples"][-1]["stderr"], "compiler failed")
            self.assertEqual(payload["error"]["sample"], 10)

    def test_main_rejects_iteration_drift(self) -> None:
        records = [record(index, iterations=2 if index == 10 else 1) for index in range(1, 11)]
        with tempfile.TemporaryDirectory() as temporary:
            result, report_path = self.run_main_with_records(Path(temporary), records)

            payload = json.loads(report_path.read_text(encoding="utf-8"))
            self.assertEqual(result, 1)
            self.assertEqual(payload["status"], "failed")
            self.assertIsNone(payload["summary"])
            self.assertIn("iteration count changed", payload["error"]["message"])

    def test_main_rejects_zero_iterations_and_keeps_parse_evidence(self) -> None:
        records = [record(index) for index in range(1, 10)]
        records.append({
            "sample": 10,
            "command": ["cargo", "bench"],
            "exit_code": 0,
            "stdout": output().replace("iterations=1", "iterations=0", 1),
            "stderr": "",
            "parse_error": "benchmark metric has no iterations",
        })
        with tempfile.TemporaryDirectory() as temporary:
            result, report_path = self.run_main_with_records(Path(temporary), records)

            payload = json.loads(report_path.read_text(encoding="utf-8"))
            self.assertEqual(result, 1)
            self.assertEqual(payload["status"], "failed")
            self.assertEqual(payload["error"]["kind"], "invalid_sample")
            self.assertIn("no iterations", payload["samples"][-1]["parse_error"])
            self.assertIsNone(payload["summary"])

    def test_main_returns_nonzero_when_complete_report_cannot_be_replaced(self) -> None:
        original_replace = Path.replace

        def fail_complete_report(path: Path, target: Path) -> Path:
            if path.name == "measurements.json.tmp":
                candidate = json.loads(path.read_text(encoding="utf-8"))
                if candidate["status"] == "complete":
                    raise OSError("read-only evidence directory")
            return original_replace(path, target)

        with tempfile.TemporaryDirectory() as temporary:
            records = [record(index) for index in range(1, 11)]
            with mock.patch.object(Path, "replace", fail_complete_report):
                result, report_path = self.run_main_with_records(Path(temporary), records)

            payload = json.loads(report_path.read_text(encoding="utf-8"))
            self.assertEqual(result, 1)
            self.assertEqual(payload["status"], "collecting")
            self.assertEqual(payload["successful_sample_count"], 10)
            self.assertIsNone(payload["summary"])

    def test_parse_requires_exactly_one_model_count_and_all_metrics(self) -> None:
        count, metrics = measure.parse_benchmark_output(output())
        self.assertEqual(count, 133)
        self.assertEqual(tuple(metrics), measure.METRICS)
        self.assertEqual(metrics[measure.METRICS[0]]["ns_per_op"], 100)

        for invalid in (
            output().replace("platform/models=133\n", ""),
            output().replace("platform/benchmark_version=3\n", ""),
            output().replace("benchmark_version=3", "benchmark_version=2"),
            output().replace("benchmark_version=3", "benchmark_version=1"),
            output() + "\nplatform/benchmark_version=3",
            output() + "\nplatform/models=133",
            output().replace("allocated_bytes/op=100", "allocated_bytes/op=broken"),
            output().replace(measure.METRICS[-1], "platform/missing_metric"),
            output() + "\n" + output().splitlines()[1],
            output().replace("iterations=1", "iterations=0", 1),
        ):
            with self.subTest(invalid=invalid[-80:]):
                with self.assertRaises(measure.MeasurementError):
                    measure.parse_benchmark_output(invalid)

    def test_summary_uses_all_fresh_samples_and_rejects_model_drift(self) -> None:
        def sample(model_count: int, ns: int) -> dict[str, object]:
            count, metrics = measure.parse_benchmark_output(output(model_count, ns))
            return {"model_count": count, "metrics": metrics}

        summary = measure.summarize_samples([sample(133, 100 + index) for index in range(10)])
        metric = summary["metrics"][measure.METRICS[0]]["ns_per_op"]
        self.assertEqual(metric, {"median": 104.5, "min": 100, "max": 109})
        self.assertEqual(summary["metrics"][measure.METRICS[0]]["iterations"], 1)
        with self.assertRaisesRegex(measure.MeasurementError, "model count changed"):
            measure.summarize_samples([sample(134 if index == 9 else 133, 100) for index in range(10)])
        with self.assertRaisesRegex(measure.MeasurementError, "iteration count changed"):
            measure.summarize_samples([
                {"model_count": 133, "metrics": measure.parse_benchmark_output(
                    output(133, 100, 2 if index == 9 else 1)
                )[1]}
                for index in range(10)
            ])
        with self.assertRaisesRegex(measure.MeasurementError, "at least ten"):
            measure.summarize_samples([sample(133, 100)])

    def test_output_must_be_under_valid_temporary_marker_and_outside_repos(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            workspace = root / "superpowers-measure-test"
            workspace.mkdir()
            marker = workspace / ".superpowers-session"
            marker.touch()
            output_dir = workspace / "results"
            repo = root / "repo"
            repo.mkdir()

            self.assertEqual(measure.validate_output_dir(output_dir, (repo,)), output_dir)
            with self.assertRaisesRegex(measure.MeasurementError, "overlaps repository"):
                measure.validate_output_dir(output_dir, (workspace,))
            marker.write_text("not empty", encoding="utf-8")
            with self.assertRaisesRegex(measure.MeasurementError, "verified"):
                measure.validate_output_dir(output_dir, (repo,))

    def test_marker_must_be_regular_and_not_a_symlink(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            workspace = root / "superpowers-measure-test"
            workspace.mkdir()
            real_marker = root / "marker"
            real_marker.touch()
            (workspace / ".superpowers-session").symlink_to(real_marker)
            with self.assertRaisesRegex(measure.MeasurementError, "verified"):
                measure.validate_output_dir(workspace / "results", ())

    def test_source_layout_uses_reflect_worktree_and_real_downstream_repositories(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "layout"
            rust_platform = source / "rust-platform"
            rust_common = source / "rust-common"
            platform = rust_platform / "rs-platform"
            model = rust_platform / "rs-model-metadata"
            reflect = root / "reflect-worktree"
            for directory in (platform, model, reflect):
                directory.mkdir(parents=True)
                (directory / "Cargo.toml").write_text("[package]\n", encoding="utf-8")
            for dependency in measure.COMMON_DEPENDENCIES:
                (rust_common / dependency).mkdir(parents=True)
            output = root / "measurements"
            output.mkdir()

            reflect_copy, model_copy, platform_copy = measure.prepare_source_layout(
                platform, output, reflect
            )

            self.assertEqual(reflect_copy.parent, model_copy.parent)
            self.assertEqual(model_copy.parent, platform_copy.parent)
            self.assertEqual((reflect_copy / "Cargo.toml").read_text(), "[package]\n")
            self.assertEqual(
                (output / "source-layout" / "rust-common" / "rs-id").resolve(),
                (rust_common / "rs-id").resolve(),
            )
            rewritten_model_manifest = (model_copy / "Cargo.toml").read_text(encoding="utf-8")
            self.assertEqual(rewritten_model_manifest, "[package]\n")

    def test_source_layout_refuses_to_overwrite_previous_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            platform = root / "rust-platform" / "rs-platform"
            platform.mkdir(parents=True)
            (platform.parent / "rs-model-metadata").mkdir()
            (root / "rust-common").mkdir()
            output = root / "measurements"
            (output / "source-layout").mkdir(parents=True)
            with self.assertRaisesRegex(measure.MeasurementError, "already exists"):
                measure.prepare_source_layout(platform, output, root / "reflect")

    def test_sample_count_cli_minimum_is_ten(self) -> None:
        with self.assertRaises(SystemExit) as raised:
            measure.parse_args(["--platform-root", ".", "--output-dir", ".", "--samples", "9"])
        self.assertEqual(raised.exception.code, 2)

    def test_repository_facts_use_stable_names_for_worktree_roots(self) -> None:
        roots = [Path(f"/tmp/{name}") for name in ("reflect-branch-name", "rs-model-metadata", "rs-platform")]
        with mock.patch.object(measure, "_git_facts", side_effect=lambda path: {"root": path.name}):
            facts = measure.repository_facts(*roots)
        self.assertEqual(set(facts), {"rs-reflect", "rs-model-metadata", "rs-platform", *measure.COMMON_DEPENDENCIES})
        self.assertEqual(facts["rs-reflect"], {"root": "reflect-branch-name"})


if __name__ == "__main__":
    unittest.main()
