#!/usr/bin/env python3
"""Measure downstream clean, cached, and edited release builds safely."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import shutil
import stat
import subprocess
import sys
from time import perf_counter_ns
from typing import Any, Sequence


AGGREGATE_TARGET_PATH = Path(
    "rust-platform/rs-platform/modules/core/src/model/aggregate_target.rs"
)
PLATFORM_MANIFEST_PATH = Path("rust-platform/rs-platform/Cargo.toml")
BINARY_NAME = "model-inventory"
EXCLUDED_NAMES = frozenset({".git", ".worktrees", "target"})
REPOSITORY_PATHS = (
    Path("rust-platform/rs-reflect"),
    Path("rust-platform/rs-model-metadata"),
    Path("rust-platform/rs-platform"),
    Path("rust-common/rs-redact"),
    Path("rust-common/rs-id"),
    Path("rust-common/rs-datatype"),
    Path("rust-common/rs-validator"),
)

METHOD_ANCHOR = """\
        self.kind.is_empty()
            && self.id.as_ref().is_none_or(|id| id == &Id::default())
            && self.property.is_empty()
"""
METHOD_REPLACEMENT = """\
        let result = {
            self.kind.is_empty()
                && self.id.as_ref().is_none_or(|id| id == &Id::default())
                && self.property.is_empty()
        };
        result
"""
FIELD_ANCHOR = """\
    pub property: Option<String>,
}
"""
FIELD_REPLACEMENT = """\
    pub property: Option<String>,

    /// Synthetic field used only to measure structural rebuild cost.
    pub benchmark_extra: Option<String>,
}
"""


class MeasurementError(RuntimeError):
    """The requested measurement cannot be performed safely."""


def _overlaps(first: Path, second: Path) -> bool:
    return first == second or first in second.parents or second in first.parents


def _verified_workspace(output: Path) -> Path | None:
    for candidate in (output, *output.parents):
        marker = candidate / ".superpowers-session"
        if not candidate.name.startswith("superpowers-") or not marker.exists():
            continue
        marker_stat = marker.lstat()
        if (
            stat.S_ISREG(marker_stat.st_mode)
            and not marker.is_symlink()
            and marker_stat.st_size == 0
        ):
            return candidate
    return None


def validate_paths(layout_root: Path, output_dir: Path) -> tuple[Path, Path]:
    """Resolve and validate the immutable input and temporary output roots."""
    layout = layout_root.resolve(strict=True)
    output = output_dir.resolve(strict=False)
    if not layout.is_dir():
        raise MeasurementError(f"layout root is not a directory: {layout}")
    if not (layout / PLATFORM_MANIFEST_PATH).is_file():
        raise MeasurementError(
            f"layout root is missing {PLATFORM_MANIFEST_PATH.as_posix()}"
        )
    if not (layout / "rust-common").is_dir():
        raise MeasurementError("layout root is missing rust-common")
    if output_dir.is_symlink():
        raise MeasurementError("output directory must not be a symbolic link")
    if _overlaps(layout, output):
        raise MeasurementError("layout root and output directory must not overlap")
    if _verified_workspace(output) is None:
        raise MeasurementError(
            "output directory must be inside a verified superpowers temporary workspace"
        )
    return layout, output


def _is_excluded(relative: Path) -> bool:
    return any(part in EXCLUDED_NAMES for part in relative.parts)


def _relevant_paths(root: Path):
    """Yield paths deterministically without descending into excluded trees."""
    for directory, directory_names, file_names in os.walk(
        root, topdown=True, followlinks=False
    ):
        directory_names[:] = sorted(
            name for name in directory_names if name not in EXCLUDED_NAMES
        )
        for name in (*directory_names, *sorted(file_names)):
            if name not in EXCLUDED_NAMES:
                yield Path(directory) / name


def _validate_source_links(layout: Path) -> None:
    for path in _relevant_paths(layout):
        relative = path.relative_to(layout)
        if not path.is_symlink():
            continue
        if Path(os.readlink(path)).is_absolute():
            raise MeasurementError(f"source tree contains absolute symlink: {relative}")
        target = path.resolve(strict=True)
        if target != layout and layout not in target.parents:
            raise MeasurementError(f"source tree contains external symlink: {relative}")


def _copy_ignore(_directory: str, names: list[str]) -> set[str]:
    return {name for name in names if name in EXCLUDED_NAMES}


def copy_layout(layout_root: Path, destination: Path) -> None:
    """Copy a layout without repository metadata or build artifacts."""
    layout = layout_root.resolve(strict=True)
    if destination.exists() or destination.is_symlink():
        raise MeasurementError(f"copy destination already exists: {destination}")
    _validate_source_links(layout)
    shutil.copytree(layout, destination, symlinks=True, ignore=_copy_ignore)


def _replace_exactly_once(path: Path, anchor: str, replacement: str) -> None:
    content = path.read_text(encoding="utf-8")
    count = content.count(anchor)
    if count != 1:
        raise MeasurementError(
            f"edit anchor must occur exactly once in {path}; found {count}"
        )
    path.write_text(content.replace(anchor, replacement, 1), encoding="utf-8")


def apply_method_edit(layout_root: Path) -> None:
    """Apply the behavior-preserving AggregateTarget::is_empty edit."""
    _replace_exactly_once(
        layout_root / AGGREGATE_TARGET_PATH,
        METHOD_ANCHOR,
        METHOD_REPLACEMENT,
    )


def apply_field_edit(layout_root: Path) -> None:
    """Add the synthetic structural field to AggregateTarget."""
    _replace_exactly_once(
        layout_root / AGGREGATE_TARGET_PATH,
        FIELD_ANCHOR,
        FIELD_REPLACEMENT,
    )


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def tree_sha256(root: Path) -> str:
    """Hash relevant paths, link targets, modes, and file contents."""
    digest = hashlib.sha256()
    for path in _relevant_paths(root):
        relative = path.relative_to(root)
        digest.update(relative.as_posix().encode("utf-8"))
        if path.is_symlink():
            digest.update(b"L")
            digest.update(os.readlink(path).encode("utf-8"))
        elif path.is_dir():
            digest.update(b"D")
        elif path.is_file():
            digest.update(b"F")
            digest.update(f"{path.stat().st_mode & 0o777:o}".encode("ascii"))
            with path.open("rb") as source:
                for chunk in iter(lambda: source.read(1024 * 1024), b""):
                    digest.update(chunk)
    return digest.hexdigest()


def _binary_size(target_dir: Path, profile: str) -> int | None:
    binary = target_dir / profile / BINARY_NAME
    return binary.stat().st_size if binary.is_file() else None


def _git_command(repository: Path, *arguments: str) -> list[str]:
    return [
        "git",
        "--no-optional-locks",
        "-C",
        str(repository),
        *arguments,
    ]


def repository_revisions(layout: Path) -> list[dict[str, Any]]:
    """Record auditable Git provenance when repository metadata is present."""
    revisions = []
    for relative in REPOSITORY_PATHS:
        repository = layout / relative
        entry: dict[str, Any] = {
            "path": relative.as_posix(),
            "present": repository.is_dir(),
            "head": None,
            "dirty": None,
            "status_sha256": None,
        }
        if repository.is_dir() and (repository / ".git").exists():
            head = subprocess.run(
                _git_command(repository, "rev-parse", "HEAD"),
                check=False,
                capture_output=True,
                text=True,
            )
            status = subprocess.run(
                _git_command(repository, "status", "--porcelain=v1"),
                check=False,
                capture_output=True,
                text=True,
            )
            if head.returncode == 0 and status.returncode == 0:
                status_text = status.stdout
                entry["head"] = head.stdout.strip()
                entry["dirty"] = bool(status_text)
                entry["status_sha256"] = hashlib.sha256(
                    status_text.encode("utf-8")
                ).hexdigest()
            else:
                entry["git_error"] = {
                    "head_exit_code": head.returncode,
                    "status_exit_code": status.returncode,
                }
        revisions.append(entry)
    return revisions


def _is_missing_benchmark_field_error(stderr: str) -> bool:
    return (
        "error[E0063]" in stderr
        and "missing field `benchmark_extra`" in stderr
    )


def _cargo_command(cargo_program: str, profile: str) -> list[str]:
    command = [
        cargo_program,
        "+1.94.0",
        "build",
        "--locked",
        "--release",
        "-p",
        "qubit-platform-testkit",
        "--bin",
        BINARY_NAME,
    ]
    if profile != "release":
        raise MeasurementError(f"unsupported profile: {profile}")
    return command


def _run_build(
    *,
    cargo_program: str,
    source_copy: Path,
    target_dir: Path,
    sample: int,
    edit_type: str,
    profile: str,
    toolchain: str,
) -> dict[str, Any]:
    command = _cargo_command(cargo_program, profile)
    environment = os.environ.copy()
    environment["CARGO_TARGET_DIR"] = str(target_dir)
    environment["CARGO_INCREMENTAL"] = "1"
    started = perf_counter_ns()
    completed = subprocess.run(
        command,
        cwd=source_copy / "rust-platform" / "rs-platform",
        env=environment,
        check=False,
        capture_output=True,
        text=True,
    )
    elapsed_ns = perf_counter_ns() - started
    return {
        "sample": sample,
        "edit_type": edit_type,
        "input_tree_sha256": tree_sha256(source_copy),
        "command": command,
        "working_directory": str(
            source_copy / "rust-platform" / "rs-platform"
        ),
        "cargo_target_dir": str(target_dir),
        "cargo_incremental": "1",
        "profile": profile,
        "toolchain": toolchain,
        "elapsed_ns": elapsed_ns,
        "binary_bytes": (
            _binary_size(target_dir, profile)
            if completed.returncode == 0
            else None
        ),
        "success": completed.returncode == 0,
        "exit_code": completed.returncode,
        "stdout": completed.stdout,
        "stderr": completed.stderr,
    }


def _persist_report(output: Path, report: dict[str, Any]) -> None:
    temporary = output / "results.json.tmp"
    temporary.write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    temporary.replace(output / "results.json")


def _toolchain_version() -> str:
    completed = subprocess.run(
        ["rustc", "+1.94.0", "-Vv"],
        check=False,
        capture_output=True,
        text=True,
    )
    if completed.returncode != 0:
        raise MeasurementError(
            "failed to query rustc +1.94.0: " + completed.stderr.strip()
        )
    return completed.stdout.strip()


def run(
    layout_root: Path,
    output_dir: Path,
    *,
    samples: int,
    profile: str,
    cargo_program: str = "cargo",
    toolchain: str | None = None,
) -> int:
    """Run measurements and return a process-style exit code."""
    if samples < 3:
        raise MeasurementError("samples must be at least 3")
    layout, output = validate_paths(layout_root, output_dir)
    if output.exists() and any(output.iterdir()):
        raise MeasurementError(f"output directory must be empty: {output}")
    output.mkdir(parents=True, exist_ok=True)
    copies_dir = output / "copies"
    targets_dir = output / "targets"
    copies_dir.mkdir()
    targets_dir.mkdir()

    aggregate_source = layout / AGGREGATE_TARGET_PATH
    source_file_hash = sha256_file(aggregate_source)
    selected_toolchain = toolchain if toolchain is not None else _toolchain_version()
    source_tree_hash = tree_sha256(layout)
    report: dict[str, Any] = {
        "schema_version": 1,
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "layout_root": str(layout),
        "output_dir": str(output),
        "samples": samples,
        "profile": profile,
        "toolchain": selected_toolchain,
        "source_tree_sha256": source_tree_hash,
        "aggregate_target_sha256_before": source_file_hash,
        "repositories": repository_revisions(layout),
        "results": [],
    }
    _persist_report(output, report)

    failed = False
    fatal_failure = False
    field_fixture_applicable = True
    for sample in range(1, samples + 1):
        scenarios = (
            ("clean-noop", None, ("clean_build", "cargo_cached_build")),
            ("method-edit", apply_method_edit, ("method_warmup", "method_body_incremental_build")),
            ("field-edit", apply_field_edit, ("field_warmup", "field_structure_incremental_build")),
        )
        for directory_name, edit, labels in scenarios:
            if directory_name == "field-edit" and not field_fixture_applicable:
                continue
            source_copy = copies_dir / f"sample-{sample:03d}-{directory_name}"
            target_dir = targets_dir / f"sample-{sample:03d}-{directory_name}"
            copy_layout(layout, source_copy)
            first = _run_build(
                cargo_program=cargo_program,
                source_copy=source_copy,
                target_dir=target_dir,
                sample=sample,
                edit_type=labels[0],
                profile=profile,
                toolchain=selected_toolchain,
            )
            report["results"].append(first)
            _persist_report(output, report)
            if not first["success"]:
                failed = True
                fatal_failure = True
                break
            if edit is not None:
                edit(source_copy)
            second = _run_build(
                cargo_program=cargo_program,
                source_copy=source_copy,
                target_dir=target_dir,
                sample=sample,
                edit_type=labels[1],
                profile=profile,
                toolchain=selected_toolchain,
            )
            if labels[1] == "field_structure_incremental_build":
                missing_field_error = _is_missing_benchmark_field_error(
                    second["stderr"]
                )
                second["fixture_applicable"] = not missing_field_error
                if missing_field_error:
                    second["fixture_failure_reason"] = (
                        "explicit initializer is missing benchmark_extra"
                    )
            report["results"].append(second)
            _persist_report(output, report)
            if not second["success"]:
                failed = True
                if (
                    labels[1] == "field_structure_incremental_build"
                    and not second["fixture_applicable"]
                ):
                    field_fixture_applicable = False
                    continue
                fatal_failure = True
                break
        if fatal_failure:
            break

    final_hash = sha256_file(aggregate_source)
    final_tree_hash = tree_sha256(layout)
    report["aggregate_target_sha256_after"] = final_hash
    report["source_tree_sha256_after"] = final_tree_hash
    report["source_unchanged"] = (
        final_hash == source_file_hash and final_tree_hash == source_tree_hash
    )
    report["field_fixture_applicable"] = field_fixture_applicable
    report["success"] = not failed and report["source_unchanged"]
    _persist_report(output, report)
    if not report["source_unchanged"]:
        raise MeasurementError("source tree changed during measurement")
    return 1 if failed else 0


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--layout-root", required=True, type=Path)
    parser.add_argument("--output-dir", required=True, type=Path)
    parser.add_argument("--samples", required=True, type=int)
    parser.add_argument("--profile", required=True, choices=("release",))
    args = parser.parse_args(argv)
    if args.samples < 3:
        parser.error("--samples must be at least 3")
    return args


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        return run(
            args.layout_root,
            args.output_dir,
            samples=args.samples,
            profile=args.profile,
        )
    except (MeasurementError, OSError) as error:
        print(f"measurement failed: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
