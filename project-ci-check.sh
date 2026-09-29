#!/bin/bash
set -euo pipefail

PROJECT_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
RS_CI_BUILD_TOOLCHAIN="${RS_CI_BUILD_TOOLCHAIN:-1.94.0}"

# Markdown examples are built in isolated workspaces with Cargo offline. Fetch
# the lockfile's complete dependency set before those checks start.
cargo +"$RS_CI_BUILD_TOOLCHAIN" fetch --locked

python3 -m unittest discover -s "$PROJECT_ROOT/scripts/tests" -p '*tests.py'
bash "$PROJECT_ROOT/scripts/tests/requirements_traceability_check_tests.sh"
bash "$PROJECT_ROOT/scripts/tests/critical_coverage_check_tests.sh"
"$PROJECT_ROOT/scripts/check-markdown-examples.sh"
"$PROJECT_ROOT/scripts/check-requirements-traceability.sh"
python3 "$PROJECT_ROOT/scripts/check_document_contracts.py"
RUSTFLAGS="${RUSTFLAGS:-} -C panic=abort" \
    cargo +"$RS_CI_BUILD_TOOLCHAIN" run --quiet --all-features \
        --bin panic_abort_invocation_fixture
echo "panic=abort invocation fixture passed."
