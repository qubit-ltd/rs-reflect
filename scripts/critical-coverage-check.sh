#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -lt 1 ] || [ "$#" -gt 3 ]; then
    echo "usage: $0 <coverage-json> [config-json] [project-root]" >&2
    exit 2
fi

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
COVERAGE_JSON="$1"
CONFIG_JSON="${2:-$SCRIPT_DIR/../.infra/ci/critical-coverage.json}"
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
    .schema_version == 2
    and (.files | type == "object")
    and (.groups | type == "object" and length > 0)
    and all(.files | keys[]; startswith("src/") and (contains("../") | not) and (contains("/./") | not))
    and all(.files[]; (keys | sort) == ["functions", "lines", "regions"])
    and all(.groups[]; (keys | sort) == ["functions", "lines", "path_prefix", "regions"])
    and all(.files[];
        (.functions | type == "number" and . >= 0 and . <= 100)
        and (.lines | type == "number" and . >= 0 and . <= 100)
        and (.regions | type == "number" and . >= 0 and . <= 100))
    and all(.groups[];
        (.path_prefix | type == "string" and startswith("src/") and endswith("/"))
        and (.functions | type == "number" and . >= 0 and . <= 100)
        and (.lines | type == "number" and . >= 0 and . <= 100)
        and (.regions | type == "number" and . >= 0 and . <= 100))
' "$CONFIG_JSON" >/dev/null; then
    echo "error: invalid critical coverage configuration (expected schema_version 2): $CONFIG_JSON" >&2
    exit 1
fi

mapfile -t group_prefixes < <(jq -r '.groups[] | .path_prefix' "$CONFIG_JSON")
for ((index = 0; index < ${#group_prefixes[@]}; index++)); do
    prefix=${group_prefixes[$index]}
    if [[ "$prefix" == *"../"* || "$prefix" == *"/./"* || "$prefix" == *$'\\'* ]]; then
        echo "error: invalid critical coverage group path prefix: $prefix" >&2
        exit 1
    fi
    for ((other = index + 1; other < ${#group_prefixes[@]}; other++)); do
        other_prefix=${group_prefixes[$other]}
        if [[ "$prefix" == "$other_prefix"* || "$other_prefix" == "$prefix"* ]]; then
            echo "error: overlapping critical coverage group prefixes: $prefix and $other_prefix" >&2
            exit 1
        fi
    done
done

status=0
while IFS=$'\t' read -r relative_file min_functions min_lines min_regions; do
    absolute_file="$PROJECT_ROOT/$relative_file"
    if ! metrics=$(jq -er --arg filename "$absolute_file" '
        [.data[].files[] | select(.filename == $filename)]
        | if length == 1 then .[0] else error("configured file is absent or duplicated") end
        | [.summary.functions.percent, .summary.lines.percent, .summary.regions.percent]
        | if all(.[]; type == "number" and . >= 0 and . <= 100)
          then @tsv
          else error("file metrics are incomplete or invalid")
          end
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

while IFS=$'\t' read -r group_name path_prefix min_functions min_lines min_regions; do
    absolute_prefix="$PROJECT_ROOT/$path_prefix"
    if ! metrics=$(jq -er --arg prefix "$absolute_prefix" '
        [.data[].files[] | select(.filename | startswith($prefix))] as $files
        | if ($files | length) == 0 then error("critical coverage group has no executable files") else . end
        | ([$files[].filename] | unique | length) as $unique_paths
        | if $unique_paths != ($files | length) then error("group contains duplicate report paths") else . end
        | (reduce $files[] as $file (
            {
                functions: {covered: 0, count: 0},
                lines: {covered: 0, count: 0},
                regions: {covered: 0, count: 0}
            };
            reduce ["functions", "lines", "regions"][] as $metric (.;
                ($file.summary[$metric].covered // error("covered count is missing")) as $covered
                | ($file.summary[$metric].count // error("total count is missing")) as $count
                | if ($covered | type) != "number"
                     or ($count | type) != "number"
                     or $count <= 0
                     or $covered < 0
                     or $covered > $count
                  then error("invalid per-file metric counts")
                  else .[$metric].covered += $covered
                       | .[$metric].count += $count
                  end
            )
        )) as $totals
        | ["functions", "lines", "regions"] as $names
        | [ $names[] as $name
            | if $totals[$name].count <= 0
                 or $totals[$name].covered < 0
                 or $totals[$name].covered > $totals[$name].count
              then error("invalid aggregate metric counts")
              else $totals[$name].covered,
                   $totals[$name].count,
                   (($totals[$name].covered * 10000 / $totals[$name].count | round) / 100)
              end
          ]
        | @tsv
    ' "$COVERAGE_JSON" 2>/dev/null); then
        echo "error: critical coverage group is absent or has invalid metrics: $group_name ($path_prefix)" >&2
        status=1
        continue
    fi

    IFS=$'\t' read -r functions_covered functions_count functions lines_covered lines_count lines regions_covered regions_count regions <<< "$metrics"
    file_count=$(jq -r --arg prefix "$absolute_prefix" '[.data[].files[] | select(.filename | startswith($prefix)) | .filename] | unique | length' "$COVERAGE_JSON")
    echo "critical coverage group: $group_name ($path_prefix, ${file_count} files)"
    echo "  actual weighted: functions=${functions_covered}/${functions_count} (${functions}%), lines=${lines_covered}/${lines_count} (${lines}%), regions=${regions_covered}/${regions_count} (${regions}%)"
    echo "  required: functions>=${min_functions}%, lines>=${min_lines}%, regions>=${min_regions}%"
    if ! jq -ne \
        --argjson functions "$functions" \
        --argjson lines "$lines" \
        --argjson regions "$regions" \
        --argjson min_functions "$min_functions" \
        --argjson min_lines "$min_lines" \
        --argjson min_regions "$min_regions" \
        '$functions >= $min_functions and $lines >= $min_lines and $regions >= $min_regions' >/dev/null; then
        echo "error: critical coverage threshold failed: $group_name ($path_prefix)" >&2
        echo "  actual weighted: functions=${functions}%, lines=${lines}%, regions=${regions}%" >&2
        echo "  required: functions>=${min_functions}%, lines>=${min_lines}%, regions>=${min_regions}%" >&2
        status=1
    else
        echo "critical coverage passed: $group_name"
    fi
done < <(jq -r '.groups | to_entries[] | [.key, .value.path_prefix, .value.functions, .value.lines, .value.regions] | @tsv' "$CONFIG_JSON")

exit "$status"
