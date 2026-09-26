#!/usr/bin/env python3
"""Collect repeatable measurements from the real downstream model benchmark."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import json
import os
import platform
from pathlib import Path
import re
import stat
import statistics
import shutil
import subprocess
import sys
from typing import Any, Sequence


METRICS = (
    "platform/cold_link_and_projection",
    "platform/type_metadata_try_of_representatives",
    "platform/warm_model_projection",
    "platform/relationship_validation",
)
METRIC_PATTERN = re.compile(
    r"^(?P<name>platform/[a-z_]+): iterations=(?P<iterations>[0-9]+), "
    r"ns/op=(?P<ns>[0-9]+), allocations/op=(?P<allocations>[0-9]+), "
    r"allocated_bytes/op=(?P<bytes>[0-9]+)$"
)
MODEL_PATTERN = re.compile(r"^platform/models=(?P<count>[0-9]+)$")
COPY_EXCLUDED = frozenset({".git", ".worktrees", "target"})
COMMON_DEPENDENCIES = (
    "rs-datatype",
    "rs-id",
    "rs-redact",
    "rs-validation-rules",
    "rs-validator",
)


class MeasurementError(RuntimeError):
    """The environment or benchmark output is not suitable for comparison."""


def parse_benchmark_output(output: str) -> tuple[int, dict[str, dict[str, int]]]:
    """Parse every required model-registry metric from one process output."""
    model_counts: list[int] = []
    metrics: dict[str, dict[str, int]] = {}
    for line in output.splitlines():
        model_match = MODEL_PATTERN.fullmatch(line.strip())
        if model_match:
            model_counts.append(int(model_match.group("count")))
            continue
        metric_match = METRIC_PATTERN.fullmatch(line.strip())
        if metric_match:
            name = metric_match.group("name")
            if name in metrics:
                raise MeasurementError(f"duplicate benchmark metric: {name}")
            metrics[name] = {
                "iterations": int(metric_match.group("iterations")),
                "ns_per_op": int(metric_match.group("ns")),
                "allocations_per_op": int(metric_match.group("allocations")),
                "allocated_bytes_per_op": int(metric_match.group("bytes")),
            }

    if len(model_counts) != 1:
        raise MeasurementError(f"expected one platform model count; found {len(model_counts)}")
    missing = [name for name in METRICS if name not in metrics]
    if missing:
        raise MeasurementError(f"missing benchmark metrics: {', '.join(missing)}")
    return model_counts[0], metrics


def summarize_samples(samples: list[dict[str, Any]]) -> dict[str, Any]:
    """Summarize samples only when all runs measured the same model set size."""
    if len(samples) < 2:
        raise MeasurementError("at least two successful samples are required")
    model_counts = {sample["model_count"] for sample in samples}
    if len(model_counts) != 1:
        raise MeasurementError(f"model count changed between samples: {sorted(model_counts)}")

    summary: dict[str, Any] = {"model_count": model_counts.pop(), "metrics": {}}
    for name in METRICS:
        summary["metrics"][name] = {}
        for field in ("ns_per_op", "allocations_per_op", "allocated_bytes_per_op"):
            values = [sample["metrics"][name][field] for sample in samples]
            summary["metrics"][name][field] = {
                "median": statistics.median(values),
                "min": min(values),
                "max": max(values),
            }
    return summary


def _inside(path: Path, parent: Path) -> bool:
    return path == parent or parent in path.parents


def validate_output_dir(output_dir: Path, repository_roots: Sequence[Path]) -> Path:
    """Require output inside a marked temporary workspace outside all repos."""
    if output_dir.is_symlink():
        raise MeasurementError("output directory must not be a symbolic link")
    output = output_dir.resolve(strict=False)
    workspace = None
    for candidate in (output, *output.parents):
        marker = candidate / ".superpowers-session"
        if not candidate.name.startswith("superpowers-") or not marker.exists():
            continue
        marker_stat = marker.lstat()
        if stat.S_ISREG(marker_stat.st_mode) and marker_stat.st_size == 0 and not marker.is_symlink():
            workspace = candidate
            break
    if workspace is None:
        raise MeasurementError("output directory must be inside a verified superpowers temporary workspace")
    if not _inside(output, workspace):
        raise MeasurementError("output directory escapes its temporary workspace")
    for root in repository_roots:
        resolved_root = root.resolve(strict=True)
        if _inside(output, resolved_root) or _inside(resolved_root, output):
            raise MeasurementError(f"output directory overlaps repository: {resolved_root}")
    return output


def _git_facts(repository: Path) -> dict[str, Any]:
    def run(*args: str) -> str:
        result = subprocess.run(
            ["git", "-C", str(repository), *args],
            check=True,
            text=True,
            capture_output=True,
        )
        return result.stdout.strip()

    return {"head": run("rev-parse", "HEAD"), "dirty": bool(run("status", "--porcelain"))}


def repository_facts(reflect_root: Path, model_root: Path, platform_root: Path) -> dict[str, Any]:
    """Use stable logical repository names, independent of worktree directory."""
    return {
        "rs-reflect": _git_facts(reflect_root),
        "rs-model-metadata": _git_facts(model_root),
        "rs-platform": _git_facts(platform_root),
    }


def _copy_repository(source: Path, destination: Path) -> None:
    shutil.copytree(
        source,
        destination,
        ignore=shutil.ignore_patterns(*COPY_EXCLUDED),
        symlinks=True,
    )


def prepare_source_layout(
    platform_root: Path,
    output: Path,
    reflect_root: Path | None = None,
) -> tuple[Path, Path, Path]:
    """Copy the three real repositories so Cargo resolves the worktree reflect crate."""
    reflect_root = reflect_root or Path(__file__).resolve().parents[1]
    model_root = platform_root.parent / "rs-model-metadata"
    layout = output / "source-layout"
    if layout.exists() or layout.is_symlink():
        raise MeasurementError(f"source layout already exists: {layout}")
    rust_platform = layout / "rust-platform"
    rust_common = layout / "rust-common"
    rust_platform.mkdir(parents=True)
    rust_common.mkdir()

    reflect_copy = rust_platform / "rs-reflect"
    model_copy = rust_platform / "rs-model-metadata"
    platform_copy = rust_platform / "rs-platform"
    _copy_repository(reflect_root, reflect_copy)
    _copy_repository(model_root, model_copy)
    _copy_repository(platform_root, platform_copy)
    for dependency in COMMON_DEPENDENCIES:
        source = platform_root.parent / dependency
        if not source.is_dir():
            raise MeasurementError(f"missing workspace path dependency: {source}")
        (rust_common / dependency).symlink_to(source.resolve(strict=True), target_is_directory=True)
    return reflect_copy, model_copy, platform_copy


def _run_once(platform_root: Path, target_dir: Path, index: int) -> dict[str, Any]:
    command = [
        "cargo",
        "+1.94.0",
        "bench",
        "--locked",
        "-p",
        "qubit-platform-testkit",
        "--bench",
        "model_registry",
    ]
    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = str(target_dir)
    result = subprocess.run(command, cwd=platform_root, env=env, text=True, capture_output=True)
    sample: dict[str, Any] = {
        "sample": index,
        "command": command,
        "exit_code": result.returncode,
        "stdout": result.stdout,
        "stderr": result.stderr,
    }
    if result.returncode != 0:
        return sample
    try:
        model_count, metrics = parse_benchmark_output(result.stdout + "\n" + result.stderr)
    except MeasurementError as error:
        sample["parse_error"] = str(error)
        return sample
    sample["model_count"] = model_count
    sample["metrics"] = metrics
    return sample


def collect(platform_root: Path, output_dir: Path, samples: int) -> Path:
    """Run fresh benchmark processes and atomically persist all raw evidence."""
    platform_root = platform_root.resolve(strict=True)
    if not (platform_root / "Cargo.toml").is_file():
        raise MeasurementError(f"platform root has no Cargo.toml: {platform_root}")
    reflect_root = Path(__file__).resolve().parents[1]
    model_root = platform_root.parent / "rs-model-metadata"
    repositories = (reflect_root, model_root, platform_root)
    output = validate_output_dir(output_dir, repositories)
    output.mkdir(parents=True, exist_ok=True)
    _, _, benchmark_platform_root = prepare_source_layout(platform_root, output)
    target_dir = output / "cargo-target"

    revision_facts = repository_facts(reflect_root, model_root, platform_root)
    records: list[dict[str, Any]] = []

    def persist() -> None:
        successful = [record for record in records if "metrics" in record]
        summary = None
        summary_error = None
        if len(successful) >= 2:
            try:
                summary = summarize_samples(successful)
            except MeasurementError as error:
                summary_error = str(error)
        payload = {
            "schema_version": 1,
            "created_utc": datetime.now(timezone.utc).isoformat(),
            "toolchain": subprocess.run(
                ["rustc", "+1.94.0", "-Vv"], check=True, text=True, capture_output=True
            ).stdout,
            "platform": {
                "system": platform.system(),
                "release": platform.release(),
                "machine": platform.machine(),
            },
            "repositories": revision_facts,
            "sample_count": samples,
            "samples": records,
            "summary": summary,
            "summary_error": summary_error,
        }
        temporary = output / "measurements.json.tmp"
        temporary.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
        temporary.replace(output / "measurements.json")

    for index in range(1, samples + 1):
        record = _run_once(benchmark_platform_root, target_dir, index)
        records.append(record)
        persist()
        if record["exit_code"] != 0:
            raise MeasurementError(f"benchmark sample {index} failed with exit code {record['exit_code']}")
        if "parse_error" in record:
            raise MeasurementError(f"benchmark sample {index}: {record['parse_error']}")
    return output / "measurements.json"


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--platform-root", required=True, type=Path)
    parser.add_argument("--output-dir", required=True, type=Path)
    parser.add_argument("--samples", required=True, type=int)
    args = parser.parse_args(argv)
    if args.samples < 10:
        parser.error("--samples must be at least 10")
    return args


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        report = collect(args.platform_root, args.output_dir, args.samples)
    except (MeasurementError, OSError, subprocess.CalledProcessError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    print(report)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
