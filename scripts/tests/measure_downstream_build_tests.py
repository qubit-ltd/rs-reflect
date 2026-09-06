#!/usr/bin/env python3
"""Focused tests for the downstream build measurement tool."""

from __future__ import annotations

import hashlib
import importlib.util
import json
from pathlib import Path
import stat
import sys
import tempfile
import unittest


SCRIPT = Path(__file__).parents[1] / "measure-downstream-build.py"
SPEC = importlib.util.spec_from_file_location("measure_downstream_build", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
measure = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = measure
SPEC.loader.exec_module(measure)


AGGREGATE_TARGET = """\
#[derive(Default)]
pub struct AggregateTarget {
    /// Property name within the aggregate root; empty when no property is
    /// scoped.
    pub property: Option<String>,
}

impl AggregateTarget {
    pub fn is_empty(&self) -> bool {
        self.kind.is_empty()
            && self.id.as_ref().is_none_or(|id| id == &Id::default())
            && self.property.is_empty()
    }
}
"""


class MeasureDownstreamBuildTests(unittest.TestCase):
    def make_layout(self, root: Path) -> Path:
        layout = root / "layout"
        source = layout / measure.AGGREGATE_TARGET_PATH
        source.parent.mkdir(parents=True)
        source.write_text(AGGREGATE_TARGET, encoding="utf-8")
        (layout / "rust-platform" / "rs-platform" / "Cargo.toml").write_text(
            "[workspace]\n", encoding="utf-8"
        )
        (layout / "rust-common").mkdir()
        return layout

    def make_workspace(self, parent: Path) -> Path:
        workspace = parent / "superpowers-build-measure-test"
        workspace.mkdir()
        (workspace / ".superpowers-session").touch()
        return workspace

    def test_edits_exactly_one_anchor_without_changing_source_tree(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            layout = self.make_layout(root)
            source = layout / measure.AGGREGATE_TARGET_PATH
            original_hash = hashlib.sha256(source.read_bytes()).hexdigest()

            method_copy = root / "method-copy"
            field_copy = root / "field-copy"
            measure.copy_layout(layout, method_copy)
            measure.copy_layout(layout, field_copy)
            measure.apply_method_edit(method_copy)
            measure.apply_field_edit(field_copy)

            self.assertIn("let result = {", (method_copy / measure.AGGREGATE_TARGET_PATH).read_text())
            self.assertIn("pub benchmark_extra: Option<String>,", (field_copy / measure.AGGREGATE_TARGET_PATH).read_text())
            self.assertEqual(hashlib.sha256(source.read_bytes()).hexdigest(), original_hash)

    def test_anchor_must_occur_exactly_once(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            layout = self.make_layout(root)
            source = layout / measure.AGGREGATE_TARGET_PATH
            source.write_text(AGGREGATE_TARGET + AGGREGATE_TARGET, encoding="utf-8")

            with self.assertRaisesRegex(measure.MeasurementError, "exactly once"):
                measure.apply_method_edit(layout)
            with self.assertRaisesRegex(measure.MeasurementError, "exactly once"):
                measure.apply_field_edit(layout)

            source.write_text("no editable anchors here\n", encoding="utf-8")
            with self.assertRaisesRegex(measure.MeasurementError, "found 0"):
                measure.apply_method_edit(layout)

    def test_rejects_overlapping_and_unverified_output_paths(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            layout = self.make_layout(root)
            workspace = self.make_workspace(root)

            with self.assertRaisesRegex(measure.MeasurementError, "overlap"):
                measure.validate_paths(layout, layout / "measurements")
            with self.assertRaisesRegex(measure.MeasurementError, "overlap"):
                measure.validate_paths(layout, root)
            with self.assertRaisesRegex(measure.MeasurementError, "temporary workspace"):
                measure.validate_paths(layout, root / "ordinary-output")

            output = workspace / "measurements"
            validated_layout, validated_output = measure.validate_paths(layout, output)
            self.assertEqual(validated_layout, layout.resolve())
            self.assertEqual(validated_output, output.resolve())

    def test_copy_rejects_links_outside_the_source_tree(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            layout = self.make_layout(root)
            outside = root / "outside"
            outside.mkdir()
            (layout / "external").symlink_to("../outside", target_is_directory=True)

            with self.assertRaisesRegex(measure.MeasurementError, "symlink"):
                measure.copy_layout(layout, root / "copy")

    def test_git_provenance_commands_disable_optional_locks(self) -> None:
        repository = Path("/tmp/example-repository")

        self.assertEqual(
            measure._git_command(repository, "status", "--porcelain=v1"),
            [
                "git",
                "--no-optional-locks",
                "-C",
                str(repository),
                "status",
                "--porcelain=v1",
            ],
        )

    def test_copy_rejects_absolute_links_into_the_source_tree(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            layout = self.make_layout(root)
            source = layout / measure.AGGREGATE_TARGET_PATH
            (layout / "absolute-internal").symlink_to(source)

            with self.assertRaisesRegex(measure.MeasurementError, "absolute symlink"):
                measure.copy_layout(layout, root / "copy")

    def test_field_fixture_failure_requires_missing_field_diagnostic(self) -> None:
        expected = "error[E0063]: missing field `benchmark_extra` in initializer"
        unrelated = "error: linking with `cc` failed"

        self.assertTrue(measure._is_missing_benchmark_field_error(expected))
        self.assertFalse(measure._is_missing_benchmark_field_error(unrelated))

    def test_failed_cargo_is_reported_and_persisted(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            layout = self.make_layout(root)
            workspace = self.make_workspace(root)
            output = workspace / "measurements"
            fake_bin = root / "bin"
            fake_bin.mkdir()
            cargo = fake_bin / "cargo"
            cargo.write_text(
                "#!/bin/sh\nprintf 'synthetic cargo failure\\n' >&2\nexit 7\n",
                encoding="utf-8",
            )
            cargo.chmod(cargo.stat().st_mode | stat.S_IXUSR)

            exit_code = measure.run(
                layout,
                output,
                samples=3,
                profile="release",
                cargo_program=str(cargo),
                toolchain="synthetic toolchain",
            )

            self.assertEqual(exit_code, 1)
            report = json.loads((output / "results.json").read_text(encoding="utf-8"))
            first = report["results"][0]
            self.assertEqual(first["exit_code"], 7)
            self.assertFalse(first["success"])
            self.assertIn("synthetic cargo failure", first["stderr"])
            self.assertEqual(first["command"][0], str(cargo))


if __name__ == "__main__":
    unittest.main()
