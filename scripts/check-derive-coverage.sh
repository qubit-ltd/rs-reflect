#!/usr/bin/env bash
set -euo pipefail

project_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
report_root="$project_root/target/derive-coverage"
coverage_json="$report_root/coverage.json"
summary_json="$report_root/summary.json"
html_dir="$report_root/html"
mkdir -p "$report_root"

rm -f "$coverage_json" "$summary_json"
export CARGO_TARGET_DIR="$project_root/target/derive-llvm-cov-target"
cargo +1.94.0 llvm-cov -p qubit-reflect-derive --lib --json --output-path "$coverage_json"
cargo +1.94.0 llvm-cov report --html --output-dir "$html_dir"
python3 "$project_root/scripts/derive_coverage_report.py" \
  "$coverage_json" "$project_root/derive/src" "$summary_json"
