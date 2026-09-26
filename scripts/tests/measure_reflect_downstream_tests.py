"""Tests for the real downstream benchmark measurement tool."""

from __future__ import annotations

import importlib.util
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


def output(model_count: int = 133, ns: int = 100) -> str:
    lines = [f"platform/models={model_count}"]
    for index, name in enumerate(measure.METRICS, start=1):
        lines.append(
            f"{name}: iterations={index}, ns/op={ns * index}, "
            f"allocations/op={10 * index}, allocated_bytes/op={100 * index}"
        )
    return "\n".join(lines)


class MeasureReflectDownstreamTests(unittest.TestCase):
    def test_parse_requires_exactly_one_model_count_and_all_metrics(self) -> None:
        count, metrics = measure.parse_benchmark_output(output())
        self.assertEqual(count, 133)
        self.assertEqual(tuple(metrics), measure.METRICS)
        self.assertEqual(metrics[measure.METRICS[0]]["ns_per_op"], 100)

        for invalid in (
            output().replace("platform/models=133\n", ""),
            output() + "\nplatform/models=133",
            output().replace("allocated_bytes/op=100", "allocated_bytes/op=broken"),
            output().replace(measure.METRICS[-1], "platform/missing_metric"),
            output() + "\n" + output().splitlines()[1],
        ):
            with self.subTest(invalid=invalid[-80:]):
                with self.assertRaises(measure.MeasurementError):
                    measure.parse_benchmark_output(invalid)

    def test_summary_uses_all_fresh_samples_and_rejects_model_drift(self) -> None:
        def sample(model_count: int, ns: int) -> dict[str, object]:
            count, metrics = measure.parse_benchmark_output(output(model_count, ns))
            return {"model_count": count, "metrics": metrics}

        summary = measure.summarize_samples([sample(133, 100), sample(133, 120)])
        metric = summary["metrics"][measure.METRICS[0]]["ns_per_op"]
        self.assertEqual(metric, {"median": 110, "min": 100, "max": 120})
        with self.assertRaisesRegex(measure.MeasurementError, "model count changed"):
            measure.summarize_samples([sample(133, 100), sample(134, 120)])
        with self.assertRaisesRegex(measure.MeasurementError, "at least two"):
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
            source = root / "source"
            platform = source / "rs-platform"
            model = source / "rs-model-metadata"
            reflect = root / "reflect-worktree"
            for directory in (platform, model, reflect):
                directory.mkdir(parents=True)
                (directory / "Cargo.toml").write_text("[package]\n", encoding="utf-8")
            for dependency in measure.COMMON_DEPENDENCIES:
                (source / dependency).mkdir(parents=True)
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
                (source / "rs-id").resolve(),
            )

    def test_source_layout_refuses_to_overwrite_previous_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            platform = root / "rust-platform" / "rs-platform"
            platform.mkdir(parents=True)
            (platform.parent / "rs-model-metadata").mkdir()
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
        self.assertEqual(set(facts), {"rs-reflect", "rs-model-metadata", "rs-platform"})
        self.assertEqual(facts["rs-reflect"], {"root": "reflect-branch-name"})


if __name__ == "__main__":
    unittest.main()
