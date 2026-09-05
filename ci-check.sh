#!/bin/bash
set -euo pipefail

PROJECT_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=.rs-ci/toolchains.sh
source "$PROJECT_ROOT/.rs-ci/toolchains.sh"
configure_rs_ci_toolchains

bash "$PROJECT_ROOT/scripts/tests/critical_coverage_check_tests.sh"
CARGO_TARGET_DIR="$PROJECT_ROOT/target/rs-ci" \
    cargo +"$RS_CI_BUILD_TOOLCHAIN" llvm-cov clean
env RS_CI_PROJECT_ROOT="$PROJECT_ROOT" "$PROJECT_ROOT/.rs-ci/ci-check.sh" "$@"
"$PROJECT_ROOT/scripts/critical-coverage-check.sh" \
    "$PROJECT_ROOT/target/llvm-cov/coverage.json" \
    "$PROJECT_ROOT/.rs-ci-critical-coverage.json" \
    "$PROJECT_ROOT"

"$PROJECT_ROOT/scripts/check-downstream.sh"
