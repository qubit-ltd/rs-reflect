#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repo_root}"

report_dir="target/derive-coverage"
mkdir -p "${report_dir}"
rm -f "${report_dir}/coverage.json" "${report_dir}/coverage.txt"

export CARGO_INCREMENTAL=1
cargo llvm-cov \
    --manifest-path derive/Cargo.toml \
    --locked \
    --all-features \
    --lib \
    --tests \
    --json \
    --output-path "${report_dir}/coverage.json"

cargo llvm-cov report \
    --manifest-path derive/Cargo.toml \
    --locked \
    --summary-only \
    --fail-under-lines 70 \
    --text \
    --output-path "${report_dir}/coverage.txt"
