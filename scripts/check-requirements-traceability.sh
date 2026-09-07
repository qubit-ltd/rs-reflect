#!/bin/bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
PROJECT_ROOT_INPUT="${TRACEABILITY_PROJECT_ROOT:-$SCRIPT_DIR/..}"
if ! PROJECT_ROOT=$(cd "$PROJECT_ROOT_INPUT" 2>/dev/null && pwd -P); then
    echo "error: traceability project root does not exist: $PROJECT_ROOT_INPUT" >&2
    exit 1
fi

TEMP_CHECK=$(mktemp -d "${TMPDIR:-/tmp}/rs-reflect-traceability.XXXXXX")
trap 'command rm -rf "$TEMP_CHECK"' EXIT

requirements_documents=(
    "doc/2026-08-28-qubit-reflect-requirements.zh_CN.md"
    "doc/2026-09-03-qubit-reflect-requirements.md"
)
traceability_documents=(
    "doc/2026-08-29-qubit-reflect-requirements-traceability.zh_CN.md"
    "doc/2026-09-03-qubit-reflect-requirements-traceability.md"
)
expected_count=285

extract_ids() {
    local kind=$1
    local document=$2
    if [ "$kind" = requirements ]; then
        # Requirement entries are the only bold lines that define IDs. This
        # avoids counting examples or verification commands as definitions.
        sed -n -E 's/^- \*\*(REQ-[A-Z]+-[0-9]{3})\*\*[:：].*/\1/p' "$document"
    else
        sed -n -E 's/^\| (REQ-[A-Z]+-[0-9]{3}) \|.*/\1/p' "$document"
    fi
}

check_ids() {
    local kind=$1
    local document=$2
    local label=$3
    local output="$TEMP_CHECK/$label.ids"
    extract_ids "$kind" "$document" > "$output"

    local rows unique
    rows=$(wc -l < "$output")
    unique=$(sort -u "$output" | wc -l)
    if [ "$rows" -ne "$unique" ]; then
        echo "error: $document contains duplicate $kind IDs" >&2
        sort "$output" | uniq -d >&2
        return 1
    fi
    if [ "$unique" -ne "$expected_count" ]; then
        echo "error: $document contains $unique unique $kind IDs; expected $expected_count" >&2
        return 1
    fi

    # The matrices are ordered by ID. A reordered row is a review-visible
    # change and must not silently pass as an equivalent set.
    if [ "$kind" = traceability ] && ! diff -u "$output" <(sort "$output"); then
        echo "error: $document traceability IDs are not sorted" >&2
        return 1
    fi
}

cd "$PROJECT_ROOT"

check_ids requirements "${requirements_documents[0]}" requirements-zh
check_ids requirements "${requirements_documents[1]}" requirements-en
check_ids traceability "${traceability_documents[0]}" traceability-zh
check_ids traceability "${traceability_documents[1]}" traceability-en

# Both language editions must define IDs in the same section order, and all
# four normalized sets must be exactly equal.
diff -u "$TEMP_CHECK/requirements-zh.ids" "$TEMP_CHECK/requirements-en.ids"
reference="$TEMP_CHECK/requirements-zh.ids"
sort "$reference" > "$TEMP_CHECK/reference.sorted"
for output in "$TEMP_CHECK"/*.ids; do
    sort "$output" > "$output.sorted"
    diff -u "$TEMP_CHECK/reference.sorted" "$output.sorted"
done

# Preserve the path existence gate for both language matrices. Only paths that
# can name repository files are considered; external downstream repository
# paths remain descriptive evidence and are intentionally out of scope here.
for traceability in "${traceability_documents[@]}"; do
    paths="$TEMP_CHECK/$(basename "$traceability").paths"
    rg -o '`[^`]+`' "$traceability" | sed 's/^`//;s/`$//' \
        | rg '^(src|derive|tests|test-crates|scripts|project-ci-check)' \
        | sort -u > "$paths" || :
    while IFS= read -r trace_path; do
        if [ ! -e "$trace_path" ]; then
            echo "error: traceability path does not exist: $trace_path" >&2
            exit 1
        fi
    done < "$paths"
done

rg -q '^\| REQ-SYS-008 .*scripts/check-markdown-examples\.sh' \
    doc/2026-09-03-qubit-reflect-requirements-traceability.md
rg -q '^\| REQ-ERR-00[1-3] .*tests/ui' \
    doc/2026-09-03-qubit-reflect-requirements-traceability.md
rg -q '^\| REQ-GEN-006 .*tests/descriptor/builtin_tests\.rs' \
    doc/2026-09-03-qubit-reflect-requirements-traceability.md
rg -q '^\| REQ-ACCPT-010 .*model_facade' \
    doc/2026-09-03-qubit-reflect-requirements-traceability.md

echo "Requirements and traceability documents are aligned."
