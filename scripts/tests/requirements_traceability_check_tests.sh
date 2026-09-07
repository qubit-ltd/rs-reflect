#!/bin/bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPOSITORY_ROOT=$(cd "$SCRIPT_DIR/../.." && pwd -P)
CHECKER="$REPOSITORY_ROOT/scripts/check-requirements-traceability.sh"
FIXTURE_ROOT=$(mktemp -d "${TMPDIR:-/tmp}/rs-reflect-traceability-fixtures.XXXXXX")
trap 'command rm -rf "$FIXTURE_ROOT"' EXIT

documents=(
    "doc/2026-08-28-qubit-reflect-requirements.zh_CN.md"
    "doc/2026-09-03-qubit-reflect-requirements.md"
    "doc/2026-08-29-qubit-reflect-requirements-traceability.zh_CN.md"
    "doc/2026-09-03-qubit-reflect-requirements-traceability.md"
)

copy_fixture() {
    local name=$1
    local root="$FIXTURE_ROOT/$name"
    for document in "${documents[@]}"; do
        mkdir -p "$root/$(dirname "$document")"
        command cp "$REPOSITORY_ROOT/$document" "$root/$document"
    done
    for directory in src derive tests test-crates scripts; do
        command ln -s "$REPOSITORY_ROOT/$directory" "$root/$directory"
    done
    command ln -s "$REPOSITORY_ROOT/project-ci-check.sh" "$root/project-ci-check.sh"
    printf '%s\n' "$root"
}

expect_failure() {
    local name=$1
    local root=$2
    if TRACEABILITY_PROJECT_ROOT="$root" "$CHECKER" >/dev/null 2>&1; then
        echo "error: traceability checker accepted $name fixture" >&2
        exit 1
    fi
}

passing=$(copy_fixture passing)
TRACEABILITY_PROJECT_ROOT="$passing" "$CHECKER" >/dev/null

missing=$(copy_fixture missing-id)
sed -i '/^| REQ-TYPE-030 |/d' \
    "$missing/doc/2026-09-03-qubit-reflect-requirements-traceability.md"
expect_failure "missing ID" "$missing"

duplicate=$(copy_fixture duplicate-id)
traceability="$duplicate/doc/2026-09-03-qubit-reflect-requirements-traceability.md"
rg '^\| REQ-TYPE-030 \|' "$traceability" >> "$traceability"
expect_failure "duplicate ID" "$duplicate"

reordered=$(copy_fixture reordered-id)
traceability="$reordered/doc/2026-09-03-qubit-reflect-requirements-traceability.md"
reordered_output="$traceability.reordered"
awk '
/^\| REQ-ACC-001 \|/ { sub("REQ-ACC-001", "REQ-ACC-002"); print; next }
/^\| REQ-ACC-002 \|/ { sub("REQ-ACC-002", "REQ-ACC-001") }
{ print }
' "$traceability" > "$reordered_output"
command mv "$reordered_output" "$traceability"
expect_failure "reordered ID" "$reordered"

missing_path=$(copy_fixture missing-path)
traceability="$missing_path/doc/2026-09-03-qubit-reflect-requirements-traceability.md"
sed -i 's#`src/capability/key.rs`#`src/capability/missing.rs`#' "$traceability"
expect_failure "missing path" "$missing_path"
