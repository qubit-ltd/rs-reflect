#!/usr/bin/env python3
"""Summarize llvm-cov JSON for the derive crate's semantic stages."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

STAGES = ("configure", "parse", "validate", "expand")
MINIMUM_LINE_COVERAGE = {
    "configure": 70.0,
    "parse": 85.0,
    "validate": 80.0,
    "expand": 85.0,
}


def summarize(report: dict[str, Any], source_root: Path) -> dict[str, Any]:
    """Return per-stage line coverage and reject missing or empty stages."""
    data = report.get("data")
    if not isinstance(data, list) or not data:
        raise ValueError("llvm-cov report has no data entries")
    files: dict[str, dict[str, int]] = {}
    for entry in data:
        for file_record in entry.get("files", []):
            filename = Path(file_record.get("filename", "")).resolve()
            try:
                relative = filename.relative_to(source_root.resolve()).as_posix()
            except ValueError:
                continue
            summary = file_record.get("summary", {}).get("lines", {})
            count = int(summary.get("count", 0))
            covered = int(summary.get("covered", 0))
            files[relative] = {"lines": count, "covered": covered}
    results: dict[str, Any] = {"schema_version": 1, "stages": {}}
    failures = []
    for stage in STAGES:
        stage_files = {
            name: counts
            for name, counts in sorted(files.items())
            if f"/{stage}/" in f"/{name}"
        }
        if not stage_files:
            failures.append(f"{stage}: no instrumented source files")
        covered = sum(item["covered"] for item in stage_files.values())
        lines = sum(item["lines"] for item in stage_files.values())
        if covered == 0:
            failures.append(f"{stage}: no executed lines")
        line_percent = (100.0 * covered / lines) if lines else 0.0
        if line_percent < MINIMUM_LINE_COVERAGE[stage]:
            failures.append(
                f"{stage}: {line_percent:.1f}% is below the "
                f"{MINIMUM_LINE_COVERAGE[stage]:.1f}% minimum"
            )
        results["stages"][stage] = {
            "files": stage_files,
            "line_count": lines,
            "covered_lines": covered,
            "line_percent": line_percent,
            "minimum_line_percent": MINIMUM_LINE_COVERAGE[stage],
        }
    results["status"] = "failed" if failures else "complete"
    results["errors"] = failures
    if failures:
        raise CoverageReportError(results)
    return results


class CoverageReportError(ValueError):
    """Raised when the report cannot prove that every stage ran."""

    def __init__(self, report: dict[str, Any]):
        super().__init__("; ".join(report["errors"]))
        self.report = report


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("coverage_json", type=Path)
    parser.add_argument("source_root", type=Path)
    parser.add_argument("output_json", type=Path)
    args = parser.parse_args()
    try:
        source = json.loads(args.coverage_json.read_text(encoding="utf-8"))
        result = summarize(source, args.source_root)
    except (OSError, json.JSONDecodeError, ValueError) as error:
        report = getattr(error, "report", {"schema_version": 1, "status": "failed", "errors": [str(error)]})
        args.output_json.parent.mkdir(parents=True, exist_ok=True)
        args.output_json.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
        print(f"derive coverage failed: {error}", file=sys.stderr)
        return 1
    args.output_json.parent.mkdir(parents=True, exist_ok=True)
    args.output_json.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    for stage, facts in result["stages"].items():
        print(f"{stage}: {facts['covered_lines']}/{facts['line_count']} lines ({facts['line_percent']:.1f}%)")
        print(f"  minimum: {facts['minimum_line_percent']:.1f}%")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
