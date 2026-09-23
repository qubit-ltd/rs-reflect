#!/bin/bash
set -euo pipefail

PROJECT_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
RS_CI_BUILD_TOOLCHAIN="${RS_CI_BUILD_TOOLCHAIN:-1.94.0}"

python3 -m unittest discover -s "$PROJECT_ROOT/scripts/tests" -p check_markdown_examples_tests.py
"$PROJECT_ROOT/scripts/check-markdown-examples.sh"
"$PROJECT_ROOT/scripts/check-requirements-traceability.sh"
RUSTFLAGS="${RUSTFLAGS:-} -C panic=abort" \
    cargo +"$RS_CI_BUILD_TOOLCHAIN" run --quiet --all-features \
        --bin panic_abort_invocation_fixture
echo "panic=abort invocation fixture passed."
