#!/bin/bash
# Validate the real reflection -> model metadata/derive -> platform dependency chain.
set -euo pipefail

REFLECT_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
PLATFORM_ROOT=$(cd "$REFLECT_ROOT/.." && pwd)
source "$REFLECT_ROOT/.rs-ci/toolchains.sh"
configure_rs_ci_toolchains

for repository in rs-model-metadata rs-platform; do
    if [[ ! -f "$PLATFORM_ROOT/$repository/Cargo.toml" ]]; then
        echo "error: missing downstream checkout: $PLATFORM_ROOT/$repository" >&2
        exit 1
    fi
done

if [[ ! -f "$PLATFORM_ROOT/rs-model-metadata/derive/Cargo.toml" ]]; then
    echo "error: missing model derive workspace member: $PLATFORM_ROOT/rs-model-metadata/derive" >&2
    exit 1
fi

cargo +"$RS_CI_BUILD_TOOLCHAIN" test --locked --manifest-path "$PLATFORM_ROOT/rs-model-metadata/Cargo.toml" --workspace --lib --tests
cargo +"$RS_CI_BUILD_TOOLCHAIN" check --locked --manifest-path "$PLATFORM_ROOT/rs-platform/Cargo.toml" --workspace
cargo +"$RS_CI_BUILD_TOOLCHAIN" test --locked --manifest-path "$PLATFORM_ROOT/rs-platform/Cargo.toml" -p qubit-platform-testkit
