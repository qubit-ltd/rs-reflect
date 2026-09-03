#!/bin/bash
set -euo pipefail

PROJECT_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=.rs-ci/toolchains.sh
source "$PROJECT_ROOT/.rs-ci/toolchains.sh"
configure_rs_ci_toolchains

"$PROJECT_ROOT/scripts/check-markdown-examples.sh"
"$PROJECT_ROOT/scripts/check-requirements-traceability.sh"
RUSTFLAGS="${RUSTFLAGS:-} -C panic=abort" \
    cargo +"$RS_CI_BUILD_TOOLCHAIN" run --quiet --all-features \
        --bin panic_abort_invocation_fixture
echo "panic=abort invocation fixture passed."
