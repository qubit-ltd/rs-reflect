#!/bin/bash
set -euo pipefail

PROJECT_ROOT="${RS_CI_PROJECT_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
# shellcheck source=../.rs-ci/toolchains.sh
source "$PROJECT_ROOT/.rs-ci/toolchains.sh"
configure_rs_ci_toolchains

MODE="${RS_CI_FUZZ_MODE:-smoke}"
SECONDS_PER_TARGET="${RS_CI_FUZZ_SECONDS_PER_TARGET:-10}"
MAX_LEN="${RS_CI_FUZZ_MAX_LEN:-4096}"
TEMP_WORKSPACE=""

cleanup() {
    if [ -n "$TEMP_WORKSPACE" ] && [ -d "$TEMP_WORKSPACE" ]; then
        command rm -rf "$TEMP_WORKSPACE"
    fi
}
trap cleanup EXIT

is_configured() {
    [ -f "$PROJECT_ROOT/fuzz/Cargo.toml" ] \
        && grep -Eq '^[[:space:]]*cargo-fuzz[[:space:]]*=[[:space:]]*true' \
            "$PROJECT_ROOT/fuzz/Cargo.toml"
}

if [ "${1:-}" = "--is-configured" ]; then
    is_configured
    exit $?
fi
if [ "$#" -ne 0 ]; then
    echo "usage: $0 [--is-configured]" >&2
    exit 2
fi
if ! is_configured; then
    echo "cargo-fuzz is not configured; skipping."
    exit 0
fi
case "$MODE" in
    disabled)
        echo "cargo-fuzz checks are disabled; skipping."
        exit 0
        ;;
    build-only | smoke) ;;
    *)
        echo "error: RS_CI_FUZZ_MODE must be smoke, build-only, or disabled" >&2
        exit 2
        ;;
esac
if ! command -v cargo-fuzz >/dev/null 2>&1; then
    echo "error: cargo-fuzz is required" >&2
    exit 1
fi

cd "$PROJECT_ROOT"
mapfile -t targets < <(cargo +"$RS_CI_FUZZ_TOOLCHAIN" fuzz list | awk 'NF')
if [ "${#targets[@]}" -eq 0 ]; then
    echo "error: cargo-fuzz reported no targets" >&2
    exit 1
fi
if [ "$MODE" = "smoke" ]; then
    TEMP_WORKSPACE=$(mktemp -d "${TMPDIR:-/tmp}/rs-reflect-fuzz.XXXXXX")
fi

for target in "${targets[@]}"; do
    echo "==> cargo fuzz build $target"
    cargo +"$RS_CI_FUZZ_TOOLCHAIN" fuzz build "$target"
    if [ "$MODE" = "build-only" ]; then
        continue
    fi
    writable_corpus="$TEMP_WORKSPACE/corpus/$target"
    artifact_dir="$PROJECT_ROOT/fuzz/artifacts/$target"
    mkdir -p "$writable_corpus" "$artifact_dir"
    corpora=("$target" "$writable_corpus")
    if [ -d "$PROJECT_ROOT/fuzz/corpus/$target" ]; then
        corpora+=("$PROJECT_ROOT/fuzz/corpus/$target")
    fi
    cargo +"$RS_CI_FUZZ_TOOLCHAIN" fuzz run "${corpora[@]}" -- \
        "-max_total_time=$SECONDS_PER_TARGET" \
        "-max_len=$MAX_LEN" \
        "-artifact_prefix=$artifact_dir/"
done

echo "cargo-fuzz $MODE checks passed."
