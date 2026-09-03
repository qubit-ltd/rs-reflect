#!/bin/bash
set -euo pipefail

PROJECT_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
TEMP_CHECK=$(mktemp -d "${TMPDIR:-/tmp}/rs-reflect-traceability.XXXXXX")
trap 'command rm -rf "$TEMP_CHECK"' EXIT

documents=(
    "doc/2026-08-28-qubit-reflect-requirements.zh_CN.md"
    "doc/2026-09-03-qubit-reflect-requirements.md"
    "doc/2026-08-29-qubit-reflect-requirements-traceability.zh_CN.md"
    "doc/2026-09-03-qubit-reflect-requirements-traceability.md"
)

cd "$PROJECT_ROOT"
for document in "${documents[@]}"; do
    output="$TEMP_CHECK/$(basename "$document").ids"
    rg -o 'REQ-[A-Z]+-[0-9]{3}' "$document" | sort -u > "$output"
    if [ "$(wc -l < "$output")" -ne 284 ]; then
        echo "error: $document does not contain exactly 284 unique requirement IDs" >&2
        exit 1
    fi
done

reference="$TEMP_CHECK/2026-08-28-qubit-reflect-requirements.zh_CN.md.ids"
for output in "$TEMP_CHECK"/*.ids; do
    diff -u "$reference" "$output"
done

for traceability in \
    doc/2026-08-29-qubit-reflect-requirements-traceability.zh_CN.md \
    doc/2026-09-03-qubit-reflect-requirements-traceability.md; do
    rows="$TEMP_CHECK/$(basename "$traceability").rows"
    sed -n -E 's/^\| (REQ-[A-Z]+-[0-9]{3}) \|.*/\1/p' "$traceability" > "$rows"
    test "$(wc -l < "$rows")" -eq 284
    test "$(sort -u "$rows" | wc -l)" -eq 284
done

rg -o '`[^`]+`' doc/2026-09-03-qubit-reflect-requirements-traceability.md \
    | sed 's/^`//;s/`$//' \
    | rg '^(src|derive|tests|test-crates|scripts|project-ci-check)' \
    | sort -u > "$TEMP_CHECK/paths"
while IFS= read -r trace_path; do
    if [ ! -e "$trace_path" ]; then
        echo "error: traceability path does not exist: $trace_path" >&2
        exit 1
    fi
done < "$TEMP_CHECK/paths"

rg -q '^\| REQ-SYS-008 .*scripts/check-markdown-examples\.sh' \
    doc/2026-09-03-qubit-reflect-requirements-traceability.md
rg -q '^\| REQ-ERR-00[1-3] .*tests/ui' \
    doc/2026-09-03-qubit-reflect-requirements-traceability.md
rg -q '^\| REQ-GEN-006 .*tests/descriptor/builtin_tests\.rs' \
    doc/2026-09-03-qubit-reflect-requirements-traceability.md
rg -q '^\| REQ-ACCPT-010 .*model_facade' \
    doc/2026-09-03-qubit-reflect-requirements-traceability.md

echo "Requirements and traceability documents are aligned."
