#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "${script_dir}/.." && pwd)"

cd "${repo_root}"
cargo +nightly-2026-06-05 miri test --locked --no-default-features --test borrowed_value_mode_boundary_tests
