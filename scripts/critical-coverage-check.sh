#!/bin/bash
set -euo pipefail

if [ "$#" -lt 1 ] || [ "$#" -gt 3 ]; then
    echo "usage: $0 <coverage-json> [config-json] [project-root]" >&2
    exit 2
fi

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
COVERAGE_JSON="$1"
CONFIG_JSON="${2:-$SCRIPT_DIR/../.rs-ci-critical-coverage.json}"
PROJECT_ROOT_INPUT="${3:-$SCRIPT_DIR/..}"

if ! PROJECT_ROOT=$(cd "$PROJECT_ROOT_INPUT" 2>/dev/null && pwd -P); then
    echo "error: project root does not exist: $PROJECT_ROOT_INPUT" >&2
    exit 1
fi

if ! command -v jq >/dev/null 2>&1; then
    echo "error: required command 'jq' was not found" >&2
    exit 1
fi

for input in "$COVERAGE_JSON" "$CONFIG_JSON"; do
    if [ ! -f "$input" ]; then
        echo "error: required JSON file does not exist: $input" >&2
        exit 1
    fi
done

if ! jq -e '
    (.files | type) == "object"
    and all(
        .files[];
        (.functions | type) == "number"
        and (.lines | type) == "number"
        and (.regions | type) == "number"
    )
' "$CONFIG_JSON" >/dev/null; then
    echo "error: invalid critical coverage configuration: $CONFIG_JSON" >&2
    exit 1
fi

status=0
while IFS=$'\t' read -r relative_file min_functions min_lines min_regions; do
    absolute_file="$PROJECT_ROOT/$relative_file"
    if ! metrics=$(jq -er --arg filename "$absolute_file" '
        [.data[].files[] | select(.filename == $filename)]
        | if length == 1 then .[0] else error("configured file is absent or duplicated") end
        | [.summary.functions.percent, .summary.lines.percent, .summary.regions.percent]
        | @tsv
    ' "$COVERAGE_JSON" 2>/dev/null); then
        echo "error: critical coverage file is absent or duplicated: $relative_file" >&2
        status=1
        continue
    fi

    IFS=$'\t' read -r functions lines regions <<< "$metrics"
    if ! jq -ne \
        --argjson functions "$functions" \
        --argjson lines "$lines" \
        --argjson regions "$regions" \
        --argjson min_functions "$min_functions" \
        --argjson min_lines "$min_lines" \
        --argjson min_regions "$min_regions" \
        '$functions >= $min_functions and $lines >= $min_lines and $regions >= $min_regions' >/dev/null; then
        echo "error: critical coverage threshold failed: $relative_file" >&2
        echo "  actual: functions=${functions}%, lines=${lines}%, regions=${regions}%" >&2
        echo "  required: functions>=${min_functions}%, lines>=${min_lines}%, regions>=${min_regions}%" >&2
        status=1
    else
        echo "critical coverage passed: $relative_file"
    fi
done < <(jq -r '.files | to_entries[] | [.key, .value.functions, .value.lines, .value.regions] | @tsv' "$CONFIG_JSON")

exit "$status"
