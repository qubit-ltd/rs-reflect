#!/bin/bash
# Validate the real reflection -> model metadata/derive -> platform dependency chain.
set -euo pipefail

REFLECT_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
PLATFORM_ROOT=$(cd "$REFLECT_ROOT/.." && pwd)
LAYOUT_ROOT=$(cd "$PLATFORM_ROOT/.." && pwd)
EVIDENCE_DIR=${REFLECT_DOWNSTREAM_EVIDENCE_DIR:-}

record_evidence() {
    local overall_status=$1
    local repository_name
    local repository_path

    {
        printf 'overall_exit_code=%s\n' "$overall_status"
        printf 'features=default\n'
        printf 'platform=%s\n' "$(uname -a 2>/dev/null || printf unavailable)"
        rustc -Vv 2>&1 || true
        if [[ -n "${RS_CI_BUILD_TOOLCHAIN:-}" ]]; then
            rustc +"$RS_CI_BUILD_TOOLCHAIN" -Vv 2>&1 || true
            cargo +"$RS_CI_BUILD_TOOLCHAIN" -V 2>&1 || true
            printf 'build_toolchain=%s\n' "$RS_CI_BUILD_TOOLCHAIN"
        fi
    } > "$EVIDENCE_DIR/environment.txt"

    : > "$EVIDENCE_DIR/repositories.txt"
    : > "$EVIDENCE_DIR/missing-repositories.txt"
    while IFS='|' read -r repository_name repository_path; do
        {
            printf 'repository=%s\n' "$repository_name"
            printf 'path=%s\n' "$repository_path"
        } >> "$EVIDENCE_DIR/repositories.txt"
        if [[ ! -d "$repository_path" ]] || ! git -C "$repository_path" rev-parse --git-dir >/dev/null 2>&1; then
            printf '%s\n' "$repository_name" >> "$EVIDENCE_DIR/missing-repositories.txt"
            printf 'head=missing\nstatus=missing\n\n' >> "$EVIDENCE_DIR/repositories.txt"
            continue
        fi
        printf 'head=%s\n' "$(git -C "$repository_path" rev-parse HEAD 2>/dev/null || printf unavailable)" \
            >> "$EVIDENCE_DIR/repositories.txt"
        printf 'status_begin\n' >> "$EVIDENCE_DIR/repositories.txt"
        git -C "$repository_path" status --porcelain 2>/dev/null \
            >> "$EVIDENCE_DIR/repositories.txt" || true
        printf 'status_end\n\n' >> "$EVIDENCE_DIR/repositories.txt"
    done <<EOF
rs-reflect|$REFLECT_ROOT
rs-model-metadata|$PLATFORM_ROOT/rs-model-metadata
rs-platform|$PLATFORM_ROOT/rs-platform
rs-id|$LAYOUT_ROOT/rust-common/rs-id
rs-datatype|$LAYOUT_ROOT/rust-common/rs-datatype
rs-redact|$LAYOUT_ROOT/rust-common/rs-redact
rs-validator|$LAYOUT_ROOT/rust-common/rs-validator
EOF

    : > "$EVIDENCE_DIR/cargo-lock-sha256.txt"
    for repository_path in \
        "$REFLECT_ROOT/Cargo.lock" \
        "$PLATFORM_ROOT/rs-model-metadata/Cargo.lock" \
        "$PLATFORM_ROOT/rs-platform/Cargo.lock"; do
        if [[ -f "$repository_path" ]]; then
            sha256sum "$repository_path" >> "$EVIDENCE_DIR/cargo-lock-sha256.txt" 2>/dev/null || true
        else
            printf 'missing  %s\n' "$repository_path" >> "$EVIDENCE_DIR/cargo-lock-sha256.txt"
        fi
    done
}

on_exit() {
    local status=$?
    set +e
    record_evidence "$status"
    trap - EXIT
    exit "$status"
}

log_and_run() {
    local status
    {
        printf 'command='
        printf '%q ' "$@"
        printf '\n'
    } >> "$EVIDENCE_DIR/commands.txt"
    set +e
    "$@"
    status=$?
    set -e
    printf 'exit_code=%s\n' "$status" >> "$EVIDENCE_DIR/commands.txt"
    return "$status"
}

if [[ -n "$EVIDENCE_DIR" ]]; then
    mkdir -p "$EVIDENCE_DIR"
    : > "$EVIDENCE_DIR/commands.txt"
    trap on_exit EXIT
fi

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

# Canonical paths keep Cargo and trybuild diagnostics stable when sibling
# checkouts are symlinks to isolated worktrees.
MODEL_ROOT=$(cd "$PLATFORM_ROOT/rs-model-metadata" && pwd -P)
DOWNSTREAM_ROOT=$(cd "$PLATFORM_ROOT/rs-platform" && pwd -P)

if [[ -n "$EVIDENCE_DIR" ]]; then
    log_and_run cargo +"$RS_CI_BUILD_TOOLCHAIN" test --locked \
        --manifest-path "$MODEL_ROOT/Cargo.toml" \
        --workspace --lib --tests
    log_and_run cargo +"$RS_CI_BUILD_TOOLCHAIN" check --locked \
        --manifest-path "$DOWNSTREAM_ROOT/Cargo.toml" --workspace
    log_and_run cargo +"$RS_CI_BUILD_TOOLCHAIN" test --locked \
        --manifest-path "$DOWNSTREAM_ROOT/Cargo.toml" \
        -p qubit-platform-testkit
else
    cargo +"$RS_CI_BUILD_TOOLCHAIN" test --locked --manifest-path "$MODEL_ROOT/Cargo.toml" --workspace --lib --tests
    cargo +"$RS_CI_BUILD_TOOLCHAIN" check --locked --manifest-path "$DOWNSTREAM_ROOT/Cargo.toml" --workspace
    cargo +"$RS_CI_BUILD_TOOLCHAIN" test --locked --manifest-path "$DOWNSTREAM_ROOT/Cargo.toml" -p qubit-platform-testkit
fi
